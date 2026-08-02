use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use serde_json::{json, Value};

use crate::bridge::callIshTerminal;

const SHELL_TERMINAL_TYPE: &str = "shell";

/// Hosts iSH Alpine Linux shell terminals embedded in the iOS application.
#[derive(Clone, Default)]
pub struct IosTerminalHost;

impl IosTerminalHost {
    /// Creates the iSH-backed iOS terminal host.
    pub fn new() -> Self {
        Self
    }
}

impl TerminalHost for IosTerminalHost {
    /// Describes the iSH terminal type available on iOS.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "ios".to_string(),
            defaultType: SHELL_TERMINAL_TYPE.to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: SHELL_TERMINAL_TYPE.to_string(),
                available: true,
                description: "iSH Alpine Linux shell".to_string(),
            }],
        })
    }

    /// Starts one typed interactive iSH terminal.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let terminalType = terminalTypeName(terminalType)?;
        let response = callIshTerminal(
            "terminalStart",
            json!({
                "sessionName": requiredText(sessionName, "session_name")?,
                "terminalType": terminalType,
                "workingDir": requiredText(workingDir, "working_directory")?,
                "rows": positiveDimension(rows, "rows")?,
                "cols": positiveDimension(cols, "cols")?,
            }),
        )?;
        requiredString(&response, "sessionId")
    }

    /// Drains raw output bytes from one iSH terminal.
    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        let response = callIshTerminal(
            "terminalRead",
            json!({"sessionId": requiredText(sessionId, "session_id")?}),
        )?;
        Ok(requiredString(&response, "output")?.into_bytes())
    }

    /// Writes raw UTF-8 terminal input to one iSH terminal.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
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

    /// Updates the screen dimensions for one iSH terminal.
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
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

    /// Returns the terminal exit status after an iSH terminal session has closed.
    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
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

    /// Closes one iSH terminal session.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        callIshTerminal(
            "terminalClose",
            json!({"sessionId": requiredText(sessionId, "session_id")?}),
        )?;
        Ok(())
    }

    /// Lists all active iSH terminal sessions.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let response = callIshTerminal("terminalList", Value::Null)?;
        let sessions = response
            .get("sessions")
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new("iSH terminal session list is invalid"))?;
        sessions.iter().map(sessionEntry).collect()
    }

    /// Reuses a named iSH terminal or creates one at the iSH root directory.
    fn createOrGetSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let response = callIshTerminal(
            "terminalCreateOrGet",
            json!({
                "sessionName": requiredText(sessionName, "session_name")?,
                "terminalType": terminalTypeName(terminalType)?,
            }),
        )?;
        Ok(TerminalSessionInfo {
            sessionId: requiredString(&response, "sessionId")?,
            sessionName: requiredString(&response, "sessionName")?,
            terminalType: terminalTypeName(&requiredString(&response, "terminalType")?)?,
            isNewSession: requiredBool(&response, "isNewSession")?,
        })
    }

    /// Executes one complete line through one iSH terminal.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        let command = requiredText(command, "command")?;
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
            terminalType: terminalTypeName(&requiredString(&response, "terminalType")?)?,
            timedOut: requiredBool(&response, "timedOut")?,
        })
    }

    /// Executes one hidden command through a persistent typed iSH terminal.
    fn executeHiddenCommand(
        &self,
        command: &str,
        terminalType: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let executorKey = requiredText(executorKey, "executor_key")?;
        let session = self.createOrGetSession(&format!("hidden:{executorKey}"), terminalType)?;
        let result = self.executeInSession(&session.sessionId, command, timeoutMs)?;
        Ok(HiddenTerminalCommandOutput {
            command: result.command,
            output: result.output,
            exitCode: result.exitCode,
            executorKey,
            terminalType: result.terminalType,
            timedOut: result.timedOut,
        })
    }

    /// Sends UTF-8 text or an explicit terminal control sequence to one session.
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

    /// Closes an iSH terminal and returns its close result.
    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let sessionId = requiredText(sessionId, "session_id")?;
        self.closePtySession(&sessionId)?;
        Ok(TerminalCloseOutput {
            sessionId,
            success: true,
            message: "iSH terminal session closed".to_string(),
        })
    }

    /// Returns the retained screen contents for one iSH terminal.
    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        let response = callIshTerminal(
            "terminalScreen",
            json!({"sessionId": requiredText(sessionId, "session_id")?}),
        )?;
        Ok(TerminalScreenOutput {
            sessionId: requiredString(&response, "sessionId")?,
            terminalType: terminalTypeName(&requiredString(&response, "terminalType")?)?,
            rows: requiredUsize(&response, "rows")?,
            cols: requiredUsize(&response, "cols")?,
            content: requiredString(&response, "content")?,
            commandRunning: requiredBool(&response, "commandRunning")?,
        })
    }
}

/// Converts one native bridge session object into the shared terminal session model.
fn sessionEntry(value: &Value) -> HostResult<TerminalSessionListEntry> {
    Ok(TerminalSessionListEntry {
        sessionId: requiredString(value, "sessionId")?,
        sessionName: requiredString(value, "sessionName")?,
        terminalType: terminalTypeName(&requiredString(value, "terminalType")?)?,
        sessionKind: requiredString(value, "sessionKind")?,
        workingDir: requiredString(value, "workingDir")?,
        commandRunning: requiredBool(value, "commandRunning")?,
    })
}

/// Validates the terminal type implemented by the iSH runtime.
fn terminalTypeName(value: &str) -> HostResult<String> {
    match value.trim() {
        SHELL_TERMINAL_TYPE => Ok(SHELL_TERMINAL_TYPE.to_string()),
        other => Err(HostError::new(format!("unsupported iSH terminal type: {other}"))),
    }
}

/// Validates a required non-blank request field.
fn requiredText(value: &str, field: &str) -> HostResult<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        Err(HostError::new(format!(
            "iSH terminal {field} must not be blank"
        )))
    } else {
        Ok(normalized.to_string())
    }
}

/// Validates a positive terminal geometry dimension.
fn positiveDimension(value: u16, field: &str) -> HostResult<u16> {
    if value == 0 {
        Err(HostError::new(format!(
            "iSH terminal {field} must be positive"
        )))
    } else {
        Ok(value)
    }
}

/// Reads a required string field from one native bridge result object.
fn requiredString(value: &Value, field: &str) -> HostResult<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| HostError::new(format!("iSH terminal result has invalid {field}")))
}

/// Reads a required Boolean field from one native bridge result object.
fn requiredBool(value: &Value, field: &str) -> HostResult<bool> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| HostError::new(format!("iSH terminal result has invalid {field}")))
}

/// Reads a required signed 32-bit integer field from one native bridge result object.
fn requiredI32(value: &Value, field: &str) -> HostResult<i32> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| HostError::new(format!("iSH terminal result has invalid {field}")))
}

/// Reads a required unsigned platform-size integer field from one native bridge result object.
fn requiredUsize(value: &Value, field: &str) -> HostResult<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| HostError::new(format!("iSH terminal result has invalid {field}")))
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
