use operit_host_api::{
    HiddenTerminalCommandOutput, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_linux_native::LinuxTerminalHost;

#[derive(Clone, Default)]
pub struct AppleTerminalHost {
    inner: LinuxTerminalHost,
}

impl AppleTerminalHost {
    pub fn new() -> Self {
        Self {
            inner: LinuxTerminalHost::new(),
        }
    }

    fn nativeTerminalType(terminalType: &str) -> &str {
        match terminalType.trim() {
            "" | "macos" | "linux" => "linux",
            value => value,
        }
    }

    fn rewriteType(value: String) -> String {
        if value == "linux" {
            "macos".to_string()
        } else {
            value
        }
    }
}

impl TerminalHost for AppleTerminalHost {
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "macos".to_string(),
            defaultType: "macos".to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: "macos".to_string(),
                available: true,
                description: "macOS bash terminal".to_string(),
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
        self.inner
            .startPtySession(sessionName, workingDir, rows, cols)
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
                    entry.terminalType = Self::rewriteType(entry.terminalType);
                    entry
                })
                .collect()
        })
    }

    fn createOrGetSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        self.inner
            .createOrGetSession(sessionName, Self::nativeTerminalType(terminalType))
            .map(|mut info| {
                info.terminalType = Self::rewriteType(info.terminalType);
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
                output.terminalType = Self::rewriteType(output.terminalType);
                output
            })
    }

    fn executeHiddenCommand(
        &self,
        command: &str,
        terminalType: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        self.inner
            .executeHiddenCommand(
                command,
                Self::nativeTerminalType(terminalType),
                executorKey,
                timeoutMs,
            )
            .map(|mut output| {
                output.terminalType = Self::rewriteType(output.terminalType);
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
            output.terminalType = Self::rewriteType(output.terminalType);
            output
        })
    }
}
