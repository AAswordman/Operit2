use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_native_common::NativePtyTerminalHost;
use serde_json::{json, Value};

use crate::bridge::callIshTerminal;

const SHELL_TERMINAL_TYPE: &str = "shell";
const PLATFORM: &str = "ios";
const ISH_TERMINAL: &str = "ish";
const SYSTEM_SHELL_TERMINAL: &str = "shell";
const NATIVE_TERMINAL: &str = "native";

#[derive(Clone, Copy, PartialEq, Eq)]
enum IosTerminalBackend {
    Ish,
    SystemShell,
}

#[derive(Default)]
struct IosTerminalState {
    sessionBackends: BTreeMap<String, IosTerminalBackend>,
}

/// Hosts iSH terminals and privileged system /bin/sh terminals on iOS.
#[derive(Clone)]
pub struct IosTerminalHost {
    state: Arc<Mutex<IosTerminalState>>,
    systemShell: NativePtyTerminalHost,
    systemShellAvailable: bool,
}

impl Default for IosTerminalHost {
    /// Creates the iOS terminal host using a real system-shell capability probe.
    fn default() -> Self {
        Self::new()
    }
}

impl IosTerminalHost {
    /// Creates the iOS terminal host and checks whether /bin/sh can actually start in a PTY.
    pub fn new() -> Self {
        let systemShell = NativePtyTerminalHost::systemShell();
        let systemShellAvailable = systemShell.probe("/").is_ok();
        Self {
            state: Arc::new(Mutex::new(IosTerminalState::default())),
            systemShell,
            systemShellAvailable,
        }
    }

    /// Mounts an App-owned directory into the embedded iSH filesystem for MCP execution.
    pub fn mountManagedRuntimeDirectory(
        &self,
        hostDirectory: &str,
        mountPoint: &str,
    ) -> HostResult<()> {
        callIshTerminal(
            "managedRuntimeMount",
            json!({
                "hostDirectory": requiredText(hostDirectory, "host_directory")?,
                "mountPoint": requiredText(mountPoint, "mount_point")?,
            }),
        )?;
        Ok(())
    }

    /// Selects iSH as the parameter-free terminal implementation shared across iOS devices.
    fn primaryBackend(&self) -> IosTerminalBackend {
        IosTerminalBackend::Ish
    }

    /// Resolves one manually selected terminal implementation and type to its backend.
    fn requestedBackend(
        &self,
        terminal: &str,
        terminalType: &str,
    ) -> HostResult<IosTerminalBackend> {
        match (terminal.trim(), terminalType.trim()) {
            (ISH_TERMINAL, SHELL_TERMINAL_TYPE) => Ok(IosTerminalBackend::Ish),
            (SYSTEM_SHELL_TERMINAL, SHELL_TERMINAL_TYPE) if self.systemShellAvailable => {
                Ok(IosTerminalBackend::SystemShell)
            }
            (SYSTEM_SHELL_TERMINAL, SHELL_TERMINAL_TYPE) => Err(HostError::new(
                "iOS system /bin/sh is unavailable because this host cannot start a privileged PTY",
            )),
            (implementation, kind) => Err(HostError::new(format!(
                "Unsupported iOS terminal implementation and type: {implementation}/{kind}"
            ))),
        }
    }

    /// Returns the public terminal implementation name for one iOS backend.
    fn terminalName(backend: IosTerminalBackend) -> &'static str {
        match backend {
            IosTerminalBackend::Ish => ISH_TERMINAL,
            IosTerminalBackend::SystemShell => SYSTEM_SHELL_TERMINAL,
        }
    }

    /// Stores the owner backend for one session identifier.
    fn recordSession(&self, sessionId: &str, backend: IosTerminalBackend) -> HostResult<()> {
        let mut state = self.lockState()?;
        state.sessionBackends.insert(sessionId.to_string(), backend);
        Ok(())
    }

    /// Removes the owner backend registration for one closed session identifier.
    fn removeSession(&self, sessionId: &str) -> HostResult<()> {
        let mut state = self.lockState()?;
        state.sessionBackends.remove(sessionId);
        Ok(())
    }

    /// Resolves the explicitly registered backend for one existing session identifier.
    fn sessionBackend(&self, sessionId: &str) -> HostResult<IosTerminalBackend> {
        let state = self.lockState()?;
        state
            .sessionBackends
            .get(sessionId)
            .copied()
            .ok_or_else(|| {
                HostError::new(format!(
                    "iOS terminal session backend is not registered: {sessionId}"
                ))
            })
    }

    /// Locks the session-backend registry.
    fn lockState(&self) -> HostResult<std::sync::MutexGuard<'_, IosTerminalState>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("iOS terminal state mutex poisoned"))
    }

    /// Starts a PTY session in one exact iOS terminal backend.
    fn startSession(
        &self,
        backend: IosTerminalBackend,
        sessionName: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let sessionName = requiredText(sessionName, "session_name")?;
        let workingDir = requiredText(workingDir, "working_directory")?;
        let rows = positiveDimension(rows, "rows")?;
        let cols = positiveDimension(cols, "cols")?;
        let sessionId = match backend {
            IosTerminalBackend::Ish => {
                let response = callIshTerminal(
                    "terminalStart",
                    json!({
                        "sessionName": sessionName,
                        "terminalType": SHELL_TERMINAL_TYPE,
                        "workingDir": workingDir,
                        "rows": rows,
                        "cols": cols,
                    }),
                )?;
                requiredString(&response, "sessionId")?
            }
            IosTerminalBackend::SystemShell => self.systemShell.startPtySession(
                &sessionName,
                NATIVE_TERMINAL,
                SHELL_TERMINAL_TYPE,
                &workingDir,
                rows,
                cols,
            )?,
        };
        self.recordSession(&sessionId, backend)?;
        Ok(sessionId)
    }

    /// Creates or reuses the backend-default persistent session for a plugin executor.
    fn createOrGetBackendSession(
        &self,
        backend: IosTerminalBackend,
        sessionName: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let sessionName = requiredText(sessionName, "session_name")?;
        let data = match backend {
            IosTerminalBackend::Ish => {
                let response = callIshTerminal(
                    "terminalCreateOrGet",
                    json!({
                        "sessionName": sessionName,
                        "terminalType": SHELL_TERMINAL_TYPE,
                    }),
                )?;
                TerminalSessionInfo {
                    sessionId: requiredString(&response, "sessionId")?,
                    sessionName: requiredString(&response, "sessionName")?,
                    platform: PLATFORM.to_string(),
                    terminal: ISH_TERMINAL.to_string(),
                    terminalType: shellTerminalType(&requiredString(&response, "terminalType")?)?,
                    isNewSession: requiredBool(&response, "isNewSession")?,
                }
            }
            IosTerminalBackend::SystemShell => {
                systemSessionInfo(self.systemShell.createOrGetSession(&sessionName)?)
            }
        };
        self.recordSession(&data.sessionId, backend)?;
        Ok(data)
    }

    /// Executes one complete command in an already registered iOS terminal session.
    fn executeSession(
        &self,
        backend: IosTerminalBackend,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        let command = requiredText(command, "command")?;
        match backend {
            IosTerminalBackend::Ish => {
                let response = callIshTerminal(
                    "terminalExecute",
                    json!({
                        "sessionId": requiredText(sessionId, "session_id")?,
                        "command": command,
                        "timeoutMs": timeoutMs,
                    }),
                )?;
                Ok(TerminalCommandOutput {
                    command,
                    output: requiredString(&response, "output")?,
                    exitCode: requiredI32(&response, "exitCode")?,
                    sessionId: requiredString(&response, "sessionId")?,
                    platform: PLATFORM.to_string(),
                    terminal: ISH_TERMINAL.to_string(),
                    terminalType: shellTerminalType(&requiredString(&response, "terminalType")?)?,
                    timedOut: requiredBool(&response, "timedOut")?,
                })
            }
            IosTerminalBackend::SystemShell => systemCommandOutput(
                self.systemShell
                    .executeInSession(sessionId, &command, timeoutMs)?,
            ),
        }
    }
}

impl TerminalHost for IosTerminalHost {
    /// Describes iSH and the real system shell capability, including privileged availability.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        let primaryBackend = self.primaryBackend();
        let primaryTerminal = Self::terminalName(primaryBackend).to_string();
        let systemShell = TerminalTypeInfo {
            terminal: SYSTEM_SHELL_TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            available: self.systemShellAvailable,
            description: "iOS system /bin/sh; requires a jailbroken or otherwise privileged host"
                .to_string(),
        };
        let ish = TerminalTypeInfo {
            terminal: ISH_TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            available: true,
            description: "iSH Alpine Linux shell".to_string(),
        };
        let types = vec![ish, systemShell];
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: primaryTerminal,
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            types,
        })
    }

    /// Starts one manually selected iOS PTY terminal.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let backend = self.requestedBackend(terminal, terminalType)?;
        self.startSession(backend, sessionName, workingDir, rows, cols)
    }

    /// Drains raw output bytes from one iOS terminal.
    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Ish => {
                let response = callIshTerminal(
                    "terminalRead",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
                Ok(requiredString(&response, "output")?.into_bytes())
            }
            IosTerminalBackend::SystemShell => self.systemShell.readPtySession(sessionId),
        }
    }

    /// Writes raw terminal input to one iOS terminal.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Ish => {
                let input = std::str::from_utf8(data)
                    .map_err(|_| HostError::new("iSH terminal input must be UTF-8"))?;
                let response = callIshTerminal(
                    "terminalWrite",
                    json!({
                        "sessionId": requiredText(sessionId, "session_id")?,
                        "input": input,
                    }),
                )?;
                requiredUsize(&response, "acceptedChars")
            }
            IosTerminalBackend::SystemShell => self.systemShell.writePtySession(sessionId, data),
        }
    }

    /// Resizes one iOS terminal PTY.
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Ish => {
                callIshTerminal(
                    "terminalResize",
                    json!({
                        "sessionId": requiredText(sessionId, "session_id")?,
                        "rows": positiveDimension(rows, "rows")?,
                        "cols": positiveDimension(cols, "cols")?,
                    }),
                )?;
                Ok(())
            }
            IosTerminalBackend::SystemShell => {
                self.systemShell.resizePtySession(sessionId, rows, cols)
            }
        }
    }

    /// Returns the exit status for one iOS terminal PTY after it has closed.
    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Ish => {
                let response = callIshTerminal(
                    "terminalPoll",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
                match response.get("exitCode") {
                    Some(Value::Null) | None => Ok(None),
                    Some(Value::Number(value)) => value
                        .as_i64()
                        .and_then(|value| i32::try_from(value).ok())
                        .map(Some)
                        .ok_or_else(|| HostError::new("iSH terminal exit code is invalid")),
                    Some(_) => Err(HostError::new("iSH terminal exit code is invalid")),
                }
            }
            IosTerminalBackend::SystemShell => self.systemShell.pollPtyExitCode(sessionId),
        }
    }

    /// Closes one iOS terminal PTY and removes its backend registration.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Ish => {
                callIshTerminal(
                    "terminalClose",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
            }
            IosTerminalBackend::SystemShell => self.systemShell.closePtySession(sessionId)?,
        }
        self.removeSession(sessionId)
    }

    /// Lists active iSH and system-shell sessions with their exact terminal identities.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let ishResponse = callIshTerminal("terminalList", Value::Null)?;
        let ishSessions = ishResponse
            .get("sessions")
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new("iSH terminal session list is invalid"))?;
        let mut sessions = Vec::with_capacity(ishSessions.len());
        for value in ishSessions {
            let session = ishSessionEntry(value)?;
            self.recordSession(&session.sessionId, IosTerminalBackend::Ish)?;
            sessions.push(session);
        }
        if self.systemShellAvailable {
            for session in self.systemShell.listSessions()? {
                let session = systemSessionListEntry(session);
                self.recordSession(&session.sessionId, IosTerminalBackend::SystemShell)?;
                sessions.push(session);
            }
        }
        Ok(sessions)
    }

    /// Creates or reuses the Host-selected terminal backend for a plugin session.
    fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetBackendSession(self.primaryBackend(), sessionName)
    }

    /// Executes one complete command in the terminal backend registered for the session.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        self.executeSession(
            self.sessionBackend(sessionId)?,
            sessionId,
            command,
            timeoutMs,
        )
    }

    /// Executes a hidden command through the Host-selected persistent terminal backend.
    fn executeHiddenCommand(
        &self,
        command: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let executorKey = requiredText(executorKey, "executor_key")?;
        let session = self.createOrGetSession(&format!("hidden:{executorKey}"))?;
        let result = self.executeSession(
            self.sessionBackend(&session.sessionId)?,
            &session.sessionId,
            command,
            timeoutMs,
        )?;
        Ok(HiddenTerminalCommandOutput {
            command: result.command,
            output: result.output,
            exitCode: result.exitCode,
            executorKey,
            platform: result.platform,
            terminal: result.terminal,
            terminalType: result.terminalType,
            timedOut: result.timedOut,
        })
    }

    /// Sends UTF-8 text or one named control sequence to a registered iOS session.
    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        let content = match (input, control) {
            (Some(value), None) => value,
            (None, Some(value)) => controlSequence(value)?,
            (Some(_), Some(_)) => {
                return Err(HostError::new(
                    "iOS terminal input accepts either text or one control sequence",
                ));
            }
            (None, None) => {
                return Err(HostError::new(
                    "iOS terminal input requires text or one control sequence",
                ));
            }
        };
        let acceptedChars = self.writePtySession(sessionId, content.as_bytes())?;
        Ok(TerminalInputOutput {
            sessionId: sessionId.to_string(),
            acceptedChars,
        })
    }

    /// Closes one iOS terminal session and returns its public close response.
    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let sessionId = requiredText(sessionId, "session_id")?;
        self.closePtySession(&sessionId)?;
        Ok(TerminalCloseOutput {
            sessionId,
            success: true,
            message: "iOS terminal session closed".to_string(),
        })
    }

    /// Returns the retained screen model for one registered iOS terminal session.
    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Ish => {
                let response = callIshTerminal(
                    "terminalScreen",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
                Ok(TerminalScreenOutput {
                    sessionId: requiredString(&response, "sessionId")?,
                    platform: PLATFORM.to_string(),
                    terminal: ISH_TERMINAL.to_string(),
                    terminalType: shellTerminalType(&requiredString(&response, "terminalType")?)?,
                    rows: requiredUsize(&response, "rows")?,
                    cols: requiredUsize(&response, "cols")?,
                    content: requiredString(&response, "content")?,
                    commandRunning: requiredBool(&response, "commandRunning")?,
                })
            }
            IosTerminalBackend::SystemShell => {
                systemScreenOutput(self.systemShell.getSessionScreen(sessionId)?)
            }
        }
    }
}

/// Applies the public iOS system-shell identity to one shared PTY session response.
fn systemSessionInfo(mut data: TerminalSessionInfo) -> TerminalSessionInfo {
    data.platform = PLATFORM.to_string();
    data.terminal = SYSTEM_SHELL_TERMINAL.to_string();
    data.terminalType = SHELL_TERMINAL_TYPE.to_string();
    data
}

/// Applies the public iOS system-shell identity to one shared PTY command response.
fn systemCommandOutput(mut data: TerminalCommandOutput) -> HostResult<TerminalCommandOutput> {
    if data.terminalType != SHELL_TERMINAL_TYPE {
        return Err(HostError::new(
            "system /bin/sh returned an unexpected terminal type",
        ));
    }
    data.platform = PLATFORM.to_string();
    data.terminal = SYSTEM_SHELL_TERMINAL.to_string();
    Ok(data)
}

/// Applies the public iOS system-shell identity to one shared PTY session-list entry.
fn systemSessionListEntry(mut data: TerminalSessionListEntry) -> TerminalSessionListEntry {
    data.platform = PLATFORM.to_string();
    data.terminal = SYSTEM_SHELL_TERMINAL.to_string();
    data.terminalType = SHELL_TERMINAL_TYPE.to_string();
    data
}

/// Applies the public iOS system-shell identity to one shared PTY screen response.
fn systemScreenOutput(mut data: TerminalScreenOutput) -> HostResult<TerminalScreenOutput> {
    if data.terminalType != SHELL_TERMINAL_TYPE {
        return Err(HostError::new(
            "system /bin/sh returned an unexpected terminal type",
        ));
    }
    data.platform = PLATFORM.to_string();
    data.terminal = SYSTEM_SHELL_TERMINAL.to_string();
    Ok(data)
}

/// Converts one iSH bridge session object into the shared terminal session model.
fn ishSessionEntry(value: &Value) -> HostResult<TerminalSessionListEntry> {
    Ok(TerminalSessionListEntry {
        sessionId: requiredString(value, "sessionId")?,
        sessionName: requiredString(value, "sessionName")?,
        platform: PLATFORM.to_string(),
        terminal: ISH_TERMINAL.to_string(),
        terminalType: shellTerminalType(&requiredString(value, "terminalType")?)?,
        sessionKind: requiredString(value, "sessionKind")?,
        workingDir: requiredString(value, "workingDir")?,
        commandRunning: requiredBool(value, "commandRunning")?,
    })
}

/// Validates the shell semantic type implemented by both iOS terminal backends.
fn shellTerminalType(value: &str) -> HostResult<String> {
    match value.trim() {
        SHELL_TERMINAL_TYPE => Ok(SHELL_TERMINAL_TYPE.to_string()),
        other => Err(HostError::new(format!(
            "unsupported iOS terminal type: {other}"
        ))),
    }
}

/// Validates a required non-blank terminal request field.
fn requiredText(value: &str, field: &str) -> HostResult<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        Err(HostError::new(format!(
            "iOS terminal {field} must not be blank"
        )))
    } else {
        Ok(normalized.to_string())
    }
}

/// Validates a positive terminal geometry dimension.
fn positiveDimension(value: u16, field: &str) -> HostResult<u16> {
    if value == 0 {
        Err(HostError::new(format!(
            "iOS terminal {field} must be positive"
        )))
    } else {
        Ok(value)
    }
}

/// Reads a required string field from one native bridge response object.
fn requiredString(value: &Value, field: &str) -> HostResult<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| HostError::new(format!("iOS terminal result has invalid {field}")))
}

/// Reads a required Boolean field from one native bridge response object.
fn requiredBool(value: &Value, field: &str) -> HostResult<bool> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| HostError::new(format!("iOS terminal result has invalid {field}")))
}

/// Reads a required signed 32-bit integer field from one native bridge response object.
fn requiredI32(value: &Value, field: &str) -> HostResult<i32> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| HostError::new(format!("iOS terminal result has invalid {field}")))
}

/// Reads a required platform-size integer field from one native bridge response object.
fn requiredUsize(value: &Value, field: &str) -> HostResult<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| HostError::new(format!("iOS terminal result has invalid {field}")))
}

/// Maps a supported iOS terminal control name into its UTF-8 byte sequence.
fn controlSequence(control: &str) -> HostResult<&'static str> {
    match control.trim() {
        "interrupt" => Ok("\u{3}"),
        "eof" => Ok("\u{4}"),
        "newline" => Ok("\n"),
        value => Err(HostError::new(format!(
            "unsupported iOS terminal control sequence: {value}"
        ))),
    }
}
