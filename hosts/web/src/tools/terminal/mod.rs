use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use js_sys::Uint8Array;
use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use wasm_bindgen::JsValue;

use crate::common::{bytes_to_js, call_terminal, js_i64, js_usize};

const LINUX_VM_TERMINAL_TYPE: &str = "linux-vm";

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
    /// Describes the Linux VM terminal available in browser builds.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "web-linux-vm".to_string(),
            defaultType: LINUX_VM_TERMINAL_TYPE.to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: LINUX_VM_TERMINAL_TYPE.to_string(),
                available: true,
                description: "Browser-local Buildroot Linux VM terminal".to_string(),
            }],
        })
    }

    /// Starts one browser-local Linux virtual machine terminal session.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let normalizedSessionName = requiredText(sessionName, "session_name")?;
        requireLinuxVmTerminalType(terminalType)?;
        let normalizedWorkingDir = requiredText(workingDir, "working_directory")?;
        if normalizedWorkingDir != "/" {
            return Err(HostError::new(
                "browser-local Linux VM terminals currently expose only the guest root directory: /",
            ));
        }
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
            terminalType: LINUX_VM_TERMINAL_TYPE.to_string(),
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
                active.push(TerminalSessionListEntry {
                    sessionId,
                    sessionName: session.sessionName,
                    terminalType: session.terminalType,
                    sessionKind: "linux-vm".to_string(),
                    workingDir: session.workingDir,
                    commandRunning: true,
                });
            }
        }
        Ok(active)
    }

    /// Returns a named Linux VM terminal session or starts it at the guest root.
    fn createOrGetSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        requireLinuxVmTerminalType(terminalType)?;
        let normalizedSessionName = requiredText(sessionName, "session_name")?;
        let existing = TERMINAL_SESSIONS.with(|sessions| {
            sessions.borrow().iter().find_map(|(sessionId, session)| {
                (session.sessionName == normalizedSessionName).then(|| TerminalSessionInfo {
                    sessionId: sessionId.clone(),
                    sessionName: session.sessionName.clone(),
                    terminalType: session.terminalType.clone(),
                    isNewSession: false,
                })
            })
        });
        if let Some(session) = existing {
            return Ok(session);
        }
        let sessionId =
            self.startPtySession(&normalizedSessionName, LINUX_VM_TERMINAL_TYPE, "/", 24, 80)?;
        Ok(TerminalSessionInfo {
            sessionId,
            sessionName: normalizedSessionName,
            terminalType: LINUX_VM_TERMINAL_TYPE.to_string(),
            isNewSession: true,
        })
    }

    /// Rejects synchronous command execution because guest output is asynchronous.
    fn executeInSession(
        &self,
        _sessionId: &str,
        _command: &str,
        _timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        Err(HostError::new(
            "browser-local Linux VM terminals are interactive; use PTY input and output streams",
        ))
    }

    /// Rejects hidden command execution because the Linux guest exposes interactive sessions only.
    fn executeHiddenCommand(
        &self,
        _command: &str,
        _terminalType: &str,
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

    /// Rejects terminal screen snapshots because the Flutter terminal owns screen state.
    fn getSessionScreen(&self, _sessionId: &str) -> HostResult<TerminalScreenOutput> {
        Err(HostError::new(
            "browser-local Linux VM terminal screen snapshots are unavailable; consume PTY output",
        ))
    }
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
fn requireLinuxVmTerminalType(terminalType: &str) -> HostResult<()> {
    if terminalType.trim() == LINUX_VM_TERMINAL_TYPE {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "browser-local Linux VM terminal type must be {LINUX_VM_TERMINAL_TYPE}: {terminalType}"
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
