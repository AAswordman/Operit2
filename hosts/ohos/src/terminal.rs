use operit_host_api::{
    HiddenTerminalCommandOutput, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_native_common::NativePtyTerminalHost;

const OHOS_TERMINAL_TYPE: &str = "ohos";
const POSIX_TERMINAL_TYPE: &str = "posix";

#[derive(Clone, Default)]
pub struct OhosTerminalHost {
    inner: NativePtyTerminalHost,
}

impl OhosTerminalHost {
    /// Creates the OpenHarmony PTY terminal host.
    pub fn new() -> Self {
        Self {
            inner: NativePtyTerminalHost::new(),
        }
    }
}

impl TerminalHost for OhosTerminalHost {
    /// Returns the OpenHarmony terminal types exposed to runtime clients.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "ohos".to_string(),
            defaultType: OHOS_TERMINAL_TYPE.to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: OHOS_TERMINAL_TYPE.to_string(),
                available: true,
                description: "OpenHarmony POSIX PTY terminal".to_string(),
            }],
        })
    }

    /// Starts an OpenHarmony PTY session.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let innerType = ohosTerminalInnerType(terminalType)?;
        self.inner
            .startPtySession(sessionName, innerType, workingDir, rows, cols)
    }

    /// Reads bytes from an OpenHarmony PTY session.
    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        self.inner.readPtySession(sessionId)
    }

    /// Writes bytes to an OpenHarmony PTY session.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        self.inner.writePtySession(sessionId, data)
    }

    /// Resizes an OpenHarmony PTY session.
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        self.inner.resizePtySession(sessionId, rows, cols)
    }

    /// Polls the process exit code for an OpenHarmony PTY session.
    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        self.inner.pollPtyExitCode(sessionId)
    }

    /// Closes an OpenHarmony PTY session.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        self.inner.closePtySession(sessionId)
    }

    /// Creates or returns a named OpenHarmony terminal session.
    fn createOrGetSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let innerType = ohosTerminalInnerType(terminalType)?;
        self.inner
            .createOrGetSession(sessionName, innerType)
            .map(ohosTerminalSessionInfo)
    }

    /// Executes a command in an OpenHarmony terminal session.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        self.inner
            .executeInSession(sessionId, command, timeoutMs)
            .map(ohosTerminalCommandOutput)
    }

    /// Executes a command in a hidden OpenHarmony terminal session.
    fn executeHiddenCommand(
        &self,
        command: &str,
        terminalType: &str,
        sessionKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let innerType = ohosTerminalInnerType(terminalType)?;
        self.inner
            .executeHiddenCommand(command, innerType, sessionKey, timeoutMs)
            .map(ohosHiddenTerminalCommandOutput)
    }

    /// Sends text or control input into an OpenHarmony terminal session.
    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        self.inner.inputInSession(sessionId, input, control)
    }

    /// Closes a named OpenHarmony terminal session.
    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        self.inner.closeSession(sessionId)
    }

    /// Reads the screen content for a named OpenHarmony terminal session.
    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        self.inner
            .getSessionScreen(sessionId)
            .map(ohosTerminalScreenOutput)
    }

    /// Lists visible OpenHarmony terminal sessions.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        self.inner.listSessions().map(|sessions| {
            sessions
                .into_iter()
                .map(ohosTerminalSessionListEntry)
                .collect()
        })
    }
}

/// Maps the public OpenHarmony terminal type to the shared POSIX PTY backend.
#[allow(non_snake_case)]
fn ohosTerminalInnerType(terminalType: &str) -> HostResult<&'static str> {
    match terminalType.trim() {
        OHOS_TERMINAL_TYPE => Ok(POSIX_TERMINAL_TYPE),
        value => Err(operit_host_api::HostError::new(format!(
            "Unsupported terminal type for OpenHarmony host: {value}"
        ))),
    }
}

/// Converts shared terminal session info to OpenHarmony public metadata.
#[allow(non_snake_case)]
fn ohosTerminalSessionInfo(mut data: TerminalSessionInfo) -> TerminalSessionInfo {
    data.terminalType = OHOS_TERMINAL_TYPE.to_string();
    data
}

/// Converts shared command output to OpenHarmony public metadata.
#[allow(non_snake_case)]
fn ohosTerminalCommandOutput(mut data: TerminalCommandOutput) -> TerminalCommandOutput {
    data.terminalType = OHOS_TERMINAL_TYPE.to_string();
    data
}

/// Converts shared hidden command output to OpenHarmony public metadata.
#[allow(non_snake_case)]
fn ohosHiddenTerminalCommandOutput(
    mut data: HiddenTerminalCommandOutput,
) -> HiddenTerminalCommandOutput {
    data.terminalType = OHOS_TERMINAL_TYPE.to_string();
    data
}

/// Converts shared screen output to OpenHarmony public metadata.
#[allow(non_snake_case)]
fn ohosTerminalScreenOutput(mut data: TerminalScreenOutput) -> TerminalScreenOutput {
    data.terminalType = OHOS_TERMINAL_TYPE.to_string();
    data
}

/// Converts shared session list entries to OpenHarmony public metadata.
#[allow(non_snake_case)]
fn ohosTerminalSessionListEntry(mut data: TerminalSessionListEntry) -> TerminalSessionListEntry {
    data.terminalType = OHOS_TERMINAL_TYPE.to_string();
    data
}
