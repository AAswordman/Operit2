use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_native_common::NativePtyTerminalHost;
use serde_json::{json, Value};

use crate::bridge::{arm64RuntimeConfigurationId, callArm64Terminal, callArm64TerminalStreaming};

const SHELL_TERMINAL_TYPE: &str = "shell";
#[cfg(not(target_os = "macos"))]
const PLATFORM: &str = "ios";
#[cfg(target_os = "macos")]
const PLATFORM: &str = "macos";
const ARM64_TERMINAL: &str = "arm64";
const SYSTEM_SHELL_TERMINAL: &str = "shell";
const NATIVE_TERMINAL: &str = "native";

#[derive(Clone, Copy, PartialEq, Eq)]
enum IosTerminalBackend {
    Arm64,
    SystemShell,
}

#[derive(Default)]
struct IosTerminalState {
    arm64ConfigurationId: Option<String>,
    sessionBackends: BTreeMap<String, IosTerminalBackend>,
    sessionWrites: BTreeMap<String, Arc<Mutex<()>>>,
}

/// Hosts ARM64 terminals and privileged system /bin/sh terminals on iOS.
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

    /// Uses only the Linux backend when composed with the macOS native shell host.
    #[cfg(target_os = "macos")]
    pub(crate) fn arm64Only() -> Self {
        Self {
            state: Arc::new(Mutex::new(IosTerminalState::default())),
            systemShell: NativePtyTerminalHost::systemShell(),
            systemShellAvailable: false,
        }
    }

    /// Shares a host directory with ARM64 at the same absolute path.
    pub fn mountManagedRuntimeDirectory(
        &self,
        hostDirectory: &str,
        mountPoint: &str,
    ) -> HostResult<()> {
        callArm64Terminal(
            "managedRuntimeMount",
            json!({
                "hostDirectory": requiredText(hostDirectory, "host_directory")?,
                "mountPoint": requiredText(mountPoint, "mount_point")?,
            }),
        )?;
        Ok(())
    }

    /// Selects ARM64 as the parameter-free terminal implementation shared across iOS devices.
    fn primaryBackend(&self) -> IosTerminalBackend {
        IosTerminalBackend::Arm64
    }

    /// Resolves one manually selected terminal implementation and type to its backend.
    fn requestedBackend(
        &self,
        terminal: &str,
        terminalType: &str,
    ) -> HostResult<IosTerminalBackend> {
        match (terminal.trim(), terminalType.trim()) {
            (ARM64_TERMINAL, SHELL_TERMINAL_TYPE) => Ok(IosTerminalBackend::Arm64),
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
            IosTerminalBackend::Arm64 => ARM64_TERMINAL,
            IosTerminalBackend::SystemShell => SYSTEM_SHELL_TERMINAL,
        }
    }

    /// Stores the owner backend for one session identifier.
    fn recordSession(&self, sessionId: &str, backend: IosTerminalBackend) -> HostResult<()> {
        let mut state = self.lockState()?;
        state.sessionBackends.insert(sessionId.to_string(), backend);
        state
            .sessionWrites
            .entry(sessionId.to_string())
            .or_default();
        Ok(())
    }

    /// Removes the owner backend registration for one closed session identifier.
    fn removeSession(&self, sessionId: &str) -> HostResult<()> {
        let mut state = self.lockState()?;
        state.sessionBackends.remove(sessionId);
        state.sessionWrites.remove(sessionId);
        Ok(())
    }

    /// Drops only ARM64 registrations when native switches its root or binds.
    /// System-shell sessions belong to a separate process environment.
    fn synchronizeArm64Configuration(&self) -> HostResult<Option<String>> {
        let identifier = arm64RuntimeConfigurationId()?;
        let mut state = self.lockState()?;
        if state.arm64ConfigurationId != identifier {
            let retired: Vec<String> = state
                .sessionBackends
                .iter()
                .filter(|(_, backend)| **backend == IosTerminalBackend::Arm64)
                .map(|(sessionId, _)| sessionId.clone())
                .collect();
            for sessionId in retired {
                state.sessionBackends.remove(&sessionId);
                state.sessionWrites.remove(&sessionId);
            }
            state.arm64ConfigurationId = identifier.clone();
        }
        Ok(identifier)
    }

    /// Rejects a creation response overtaken by a concurrent native root switch.
    fn requireArm64Configuration(&self, response: &Value) -> HostResult<()> {
        let generation = requiredString(response, "configurationId")?;
        if self.synchronizeArm64Configuration()?.as_deref() != Some(generation.as_str()) {
            return Err(HostError::new(
                "ARM64 runtime configuration changed while creating the terminal; retry the launch",
            ));
        }
        Ok(())
    }

    /// Resolves an existing session without permitting a cached owner from an old root.
    fn sessionBackend(&self, sessionId: &str) -> HostResult<IosTerminalBackend> {
        let backend = self.lockState()?.sessionBackends.get(sessionId).copied();
        if backend == Some(IosTerminalBackend::Arm64) {
            self.synchronizeArm64Configuration()?;
        }
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
            IosTerminalBackend::Arm64 => {
                let response = callArm64Terminal(
                    "terminalStart",
                    json!({
                        "sessionName": sessionName,
                        "terminalType": SHELL_TERMINAL_TYPE,
                        "workingDir": workingDir,
                        "rows": rows,
                        "cols": cols,
                    }),
                )?;
                self.requireArm64Configuration(&response)?;
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
            IosTerminalBackend::Arm64 => {
                let response = callArm64Terminal(
                    "terminalCreateOrGet",
                    json!({
                        "sessionName": sessionName,
                        "terminalType": SHELL_TERMINAL_TYPE,
                    }),
                )?;
                self.requireArm64Configuration(&response)?;
                TerminalSessionInfo {
                    sessionId: requiredString(&response, "sessionId")?,
                    sessionName: requiredString(&response, "sessionName")?,
                    platform: PLATFORM.to_string(),
                    terminal: ARM64_TERMINAL.to_string(),
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
            IosTerminalBackend::Arm64 => {
                let response = callArm64Terminal(
                    "terminalExecute",
                    json!({
                        "sessionId": requiredText(sessionId, "session_id")?,
                        "command": command,
                        "timeoutMs": timeoutMs,
                    }),
                )?;
                Ok(TerminalCommandOutput {
                    command,
                    output: terminalText(&response, "outputBytes")?,
                    exitCode: requiredI32(&response, "exitCode")?,
                    sessionId: requiredString(&response, "sessionId")?,
                    platform: PLATFORM.to_string(),
                    terminal: ARM64_TERMINAL.to_string(),
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
    /// Resolves ARM64 guest paths without requiring a Linux mount on the iOS host.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        match self.requestedBackend(terminal, terminalType)? {
            IosTerminalBackend::Arm64 => {
                operit_host_api::TerminalWorkingDirectory::resolveLinuxGuestDirectory(
                    workingDir,
                    &|path| {
                        let directory = resolveHostDirectory(path)?;
                        callArm64Terminal("terminalWorkspaceMount", json!({
                            "hostDirectory": directory,
                            "mountPoint": directory,
                        }))?;
                        Ok(directory)
                    },
                )
            }
            IosTerminalBackend::SystemShell => resolveHostDirectory(workingDir),
        }
    }

    /// Describes ARM64 and the real system shell capability, including privileged availability.
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
        let arm64 = TerminalTypeInfo {
            terminal: ARM64_TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            available: true,
            description: "Operit ARM64 Linux shell using the configured root filesystem"
                .to_string(),
        };
        let types = vec![arm64, systemShell];
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
            IosTerminalBackend::Arm64 => {
                let response = callArm64Terminal(
                    "terminalRead",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
                requiredBytes(&response, "bytes")
            }
            IosTerminalBackend::SystemShell => self.systemShell.readPtySession(sessionId),
        }
    }

    /// Writes raw terminal input to one iOS terminal.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Arm64 => {
                let sessionId = requiredText(sessionId, "session_id")?;
                let writeLock = self
                    .lockState()?
                    .sessionWrites
                    .get(&sessionId)
                    .cloned()
                    .ok_or_else(|| HostError::new("ARM64 terminal writer is not registered"))?;
                let _writer = writeLock
                    .lock()
                    .map_err(|_| HostError::new("ARM64 terminal writer mutex poisoned"))?;
                // Match the shared native host's write_all behavior. Managed
                // runtime callers rely on an entire protocol frame being sent.
                // Each native request reports only bytes accepted by the PTY.
                let deadline = Instant::now() + Duration::from_secs(30);
                let mut written = 0usize;
                while written < data.len() {
                    let end = written.saturating_add(16 * 1024).min(data.len());
                    let response = callArm64Terminal(
                        "terminalWrite",
                        json!({"sessionId": sessionId, "bytes": &data[written..end]}),
                    )?;
                    let accepted = requiredUsize(&response, "acceptedBytes")?;
                    if accepted > end - written {
                        return Err(HostError::new(
                            "ARM64 terminal returned an invalid write count",
                        ));
                    }
                    written += accepted;
                    if written < data.len() && Instant::now() >= deadline {
                        return Err(HostError::new(format!(
                            "ARM64 terminal input timed out after accepting {written} of {} bytes",
                            data.len()
                        )));
                    }
                    if accepted == 0 {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
                Ok(written)
            }
            IosTerminalBackend::SystemShell => self.systemShell.writePtySession(sessionId, data),
        }
    }

    /// Resizes one iOS terminal PTY.
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Arm64 => {
                callArm64Terminal(
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
            IosTerminalBackend::Arm64 => {
                let response = callArm64Terminal(
                    "terminalPoll",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
                match response.get("exitCode") {
                    Some(Value::Null) | None => Ok(None),
                    Some(Value::Number(value)) => value
                        .as_i64()
                        .and_then(|value| i32::try_from(value).ok())
                        .map(Some)
                        .ok_or_else(|| HostError::new("ARM64 terminal exit code is invalid")),
                    Some(_) => Err(HostError::new("ARM64 terminal exit code is invalid")),
                }
            }
            IosTerminalBackend::SystemShell => self.systemShell.pollPtyExitCode(sessionId),
        }
    }

    /// Closes one iOS terminal PTY and removes its backend registration.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Arm64 => {
                callArm64Terminal(
                    "terminalClose",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
            }
            IosTerminalBackend::SystemShell => self.systemShell.closePtySession(sessionId)?,
        }
        self.removeSession(sessionId)
    }

    /// Lists active ARM64 and system-shell sessions with their exact terminal identities.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let arm64Response = callArm64Terminal("terminalList", json!({}))?;
        let arm64Sessions = arm64Response
            .get("sessions")
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new("ARM64 terminal session list is invalid"))?;
        let generation = self.synchronizeArm64Configuration()?;
        let mut sessions = Vec::with_capacity(arm64Sessions.len());
        for value in arm64Sessions {
            if value.get("configurationId").and_then(Value::as_str) != generation.as_deref() {
                continue;
            }
            let session = arm64SessionEntry(value)?;
            self.recordSession(&session.sessionId, IosTerminalBackend::Arm64)?;
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
        self.createOrGetTypedSession(sessionName, SHELL_TERMINAL_TYPE)
    }

    /// Creates or reuses the requested interpreter in the configured iOS terminal backend.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let backend = self.primaryBackend();
        let backend = self.requestedBackend(Self::terminalName(backend), terminalType)?;
        self.createOrGetBackendSession(backend, sessionName)
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

    /// Observes the real backend command loop without draining the terminal screen separately.
    fn executeInSessionStreaming(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        match self.sessionBackend(sessionId)? {
            IosTerminalBackend::Arm64 => {
                let command = requiredText(command, "command")?;
                let response = callArm64TerminalStreaming(
                    json!({
                        "sessionId": requiredText(sessionId, "session_id")?, "command": command, "timeoutMs": timeoutMs,
                    }),
                    onOutput,
                )?;
                Ok(TerminalCommandOutput {
                    command,
                    output: terminalText(&response, "outputBytes")?,
                    exitCode: requiredI32(&response, "exitCode")?,
                    sessionId: requiredString(&response, "sessionId")?,
                    platform: PLATFORM.into(),
                    terminal: ARM64_TERMINAL.into(),
                    terminalType: shellTerminalType(&requiredString(&response, "terminalType")?)?,
                    timedOut: requiredBool(&response, "timedOut")?,
                })
            }
            IosTerminalBackend::SystemShell => systemCommandOutput(
                self.systemShell
                    .executeInSessionStreaming(sessionId, command, timeoutMs, onOutput)?,
            ),
        }
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
        let acceptedChars = if control.map(str::trim) == Some("interrupt")
            && self.sessionBackend(sessionId)? == IosTerminalBackend::Arm64
        {
            callArm64Terminal(
                "terminalSignal",
                json!({"sessionId": requiredText(sessionId, "session_id")?, "signal": 2}),
            )?;
            1
        } else {
            self.writePtySession(sessionId, content.as_bytes())?
        };
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
            IosTerminalBackend::Arm64 => {
                let response = callArm64Terminal(
                    "terminalScreen",
                    json!({"sessionId": requiredText(sessionId, "session_id")?}),
                )?;
                Ok(TerminalScreenOutput {
                    sessionId: requiredString(&response, "sessionId")?,
                    platform: PLATFORM.to_string(),
                    terminal: ARM64_TERMINAL.to_string(),
                    terminalType: shellTerminalType(&requiredString(&response, "terminalType")?)?,
                    rows: requiredUsize(&response, "rows")?,
                    cols: requiredUsize(&response, "cols")?,
                    content: terminalText(&response, "contentBytes")?,
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

/// Converts one ARM64 bridge session object into the shared terminal session model.
fn arm64SessionEntry(value: &Value) -> HostResult<TerminalSessionListEntry> {
    Ok(TerminalSessionListEntry {
        sessionId: requiredString(value, "sessionId")?,
        sessionName: requiredString(value, "sessionName")?,
        platform: PLATFORM.to_string(),
        terminal: ARM64_TERMINAL.to_string(),
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

/// Preserves arbitrary PTY bytes, including UTF-8 sequences split across reads.
fn requiredBytes(value: &Value, field: &str) -> HostResult<Vec<u8>> {
    let bytes = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new(format!("ARM64 terminal result has invalid {field}")))?;
    bytes
        .iter()
        .map(|byte| {
            byte.as_u64()
                .and_then(|byte| u8::try_from(byte).ok())
                .ok_or_else(|| {
                    HostError::new(format!(
                        "ARM64 terminal result has an invalid byte in {field}"
                    ))
                })
        })
        .collect()
}

/// Public command/screen models contain text; the PTY and streaming APIs retain
/// their exact bytes and do not decode individual frames.
fn terminalText(value: &Value, field: &str) -> HostResult<String> {
    Ok(String::from_utf8_lossy(&requiredBytes(value, field)?).into_owned())
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
