use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_native_common::NativePtyTerminalHost;

const PLATFORM: &str = "macos";
const TERMINAL: &str = "native";
const TERMINAL_TYPE: &str = "bash";

#[derive(Clone, Default)]
pub struct AppleTerminalHost {
    inner: NativePtyTerminalHost,
}

impl AppleTerminalHost {
    /// Creates the macOS terminal host backed by the shared POSIX PTY engine.
    pub fn new() -> Self {
        Self {
            inner: NativePtyTerminalHost::new(),
        }
    }

    /// Resolves the public macOS terminal type to the shared PTY shell type.
    fn nativeTerminalType(terminal: &str, terminalType: &str) -> HostResult<&'static str> {
        match (terminal.trim(), terminalType.trim()) {
            (TERMINAL, TERMINAL_TYPE) => Ok(TERMINAL_TYPE),
            (implementation, value) => Err(HostError::new(format!(
                "Unsupported terminal implementation and type for macos host: {implementation}/{value}"
            ))),
        }
    }
}

impl TerminalHost for AppleTerminalHost {
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: TERMINAL_TYPE.to_string(),
            types: vec![TerminalTypeInfo {
                terminal: TERMINAL.to_string(),
                terminalType: TERMINAL_TYPE.to_string(),
                available: true,
                description: "macOS bash terminal".to_string(),
            }],
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
        self.inner.startPtySession(
            sessionName,
            TERMINAL,
            Self::nativeTerminalType(terminal, terminalType)?,
            workingDir,
            rows,
            cols,
        )
    }

    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        self.inner.readPtySession(sessionId)
    }

    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        self.inner.writePtySession(sessionId, data)
    }

    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        self.inner.resizePtySession(sessionId, rows, cols)
    }

    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        self.inner.pollPtyExitCode(sessionId)
    }

    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        self.inner.closePtySession(sessionId)
    }

    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        self.inner.listSessions().map(|entries| {
            entries
                .into_iter()
                .map(|mut entry| {
                    entry.platform = PLATFORM.to_string();
                    entry.terminal = TERMINAL.to_string();
                    entry
                })
                .collect()
        })
    }

    fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
        self.inner.createOrGetSession(sessionName).map(|mut info| {
            info.platform = PLATFORM.to_string();
            info.terminal = TERMINAL.to_string();
            info
        })
    }

    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        self.inner
            .executeInSession(sessionId, command, timeoutMs)
            .map(|mut output| {
                output.platform = PLATFORM.to_string();
                output.terminal = TERMINAL.to_string();
                output
            })
    }

    fn executeHiddenCommand(
        &self,
        command: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        self.inner
            .executeHiddenCommand(command, executorKey, timeoutMs)
            .map(|mut output| {
                output.platform = PLATFORM.to_string();
                output.terminal = TERMINAL.to_string();
                output
            })
    }

    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        self.inner.inputInSession(sessionId, input, control)
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        self.inner.closeSession(sessionId)
    }

    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        self.inner.getSessionScreen(sessionId).map(|mut output| {
            output.platform = PLATFORM.to_string();
            output.terminal = TERMINAL.to_string();
            output
        })
    }
}
