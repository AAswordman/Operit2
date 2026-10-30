use std::collections::BTreeMap;

use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_native_common::NativePtyTerminalHost;

const PLATFORM: &str = "macos";
const TERMINAL: &str = "native";

#[derive(Clone)]
pub struct AppleTerminalHost {
    hosts: BTreeMap<String, NativePtyTerminalHost>,
    primary: String,
    sandboxed: bool,
}

impl Default for AppleTerminalHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AppleTerminalHost {
    /// Registers the real login shells outside App Sandbox, or the sandbox PTY adapter.
    pub fn new() -> Self {
        let sandboxed =
            crate::sandbox::processIsolation() == operit_host_api::HostIsolation::OsAppSandbox;
        let mut hosts = BTreeMap::new();
        let mut primary = "bash".to_string();
        if sandboxed {
            hosts.insert(
                primary.clone(),
                NativePtyTerminalHost::new().withControllingTty(false),
            );
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
            for (kind, program) in [("bash", "/bin/bash"), ("zsh", "/bin/zsh")] {
                if let Ok(host) = NativePtyTerminalHost::loginShell(program, kind, &home) {
                    hosts.insert(kind.to_string(), host);
                }
            }
            let preferred = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
            let preferred = preferred.rsplit('/').next().unwrap_or("zsh");
            if hosts.contains_key(preferred) {
                primary = preferred.to_string();
            } else if hosts.contains_key("zsh") {
                primary = "zsh".to_string();
            }
        }
        Self {
            hosts,
            primary,
            sandboxed,
        }
    }

    fn hostForType(
        &self,
        terminal: &str,
        terminalType: &str,
    ) -> HostResult<&NativePtyTerminalHost> {
        if terminal.trim() == TERMINAL {
            if let Some(host) = self.hosts.get(terminalType.trim()) {
                return Ok(host);
            }
        }
        Err(HostError::new(format!(
            "Unsupported macOS terminal: {terminal}/{terminalType}"
        )))
    }

    fn hostForSession(&self, sessionId: &str) -> HostResult<&NativePtyTerminalHost> {
        for host in self.hosts.values() {
            if host.ownsSession(sessionId)? {
                return Ok(host);
            }
        }
        Err(HostError::new(format!(
            "Unknown macOS terminal session: {sessionId}"
        )))
    }
}

impl TerminalHost for AppleTerminalHost {
    /// Resolves macOS working directories through the shared native PTY host.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        self.hostForType(terminal, terminalType)?
            .resolveWorkingDirectory(TERMINAL, terminalType, workingDir, resolveHostDirectory)
    }

    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: self.primary.clone(),
            types: self
                .hosts
                .keys()
                .map(|kind| TerminalTypeInfo {
                    terminal: TERMINAL.to_string(),
                    terminalType: kind.clone(),
                    available: true,
                    description: if self.sandboxed {
                        "macOS sandbox PTY (without controlling TTY)".to_string()
                    } else {
                        format!("macOS native {kind} login PTY with job control")
                    },
                })
                .collect(),
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
        self.hostForType(terminal, terminalType)?.startPtySession(
            sessionName,
            TERMINAL,
            terminalType,
            workingDir,
            rows,
            cols,
        )
    }

    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        self.hostForSession(sessionId)?.readPtySession(sessionId)
    }

    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        self.hostForSession(sessionId)?
            .writePtySession(sessionId, data)
    }

    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        self.hostForSession(sessionId)?
            .resizePtySession(sessionId, rows, cols)
    }

    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        self.hostForSession(sessionId)?.pollPtyExitCode(sessionId)
    }

    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        self.hostForSession(sessionId)?.closePtySession(sessionId)
    }

    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let mut entries = Vec::new();
        for host in self.hosts.values() {
            entries.extend(host.listSessions()?);
        }
        Ok(entries
            .into_iter()
            .map(|mut entry| {
                entry.platform = PLATFORM.to_string();
                entry.terminal = TERMINAL.to_string();
                entry
            })
            .collect())
    }

    /// Creates or reuses a named session in the default macOS interpreter.
    fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTypedSession(sessionName, &self.primary)
    }

    /// Creates or reuses a named session of exactly the requested macOS interpreter type.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        self.hostForType(TERMINAL, terminalType)?
            .createOrGetTypedSession(sessionName, terminalType)
            .map(|mut info| {
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
        self.hostForSession(sessionId)?
            .executeInSession(sessionId, command, timeoutMs)
            .map(|mut output| {
                output.platform = PLATFORM.to_string();
                output.terminal = TERMINAL.to_string();
                output
            })
    }

    fn executeInSessionStreaming(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        self.hostForSession(sessionId)?
            .executeInSessionStreaming(sessionId, command, timeoutMs, onOutput)
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
        self.hostForType(TERMINAL, &self.primary)?
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
        self.hostForSession(sessionId)?
            .inputInSession(sessionId, input, control)
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        self.hostForSession(sessionId)?.closeSession(sessionId)
    }

    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        self.hostForSession(sessionId)?
            .getSessionScreen(sessionId)
            .map(|mut output| {
                output.platform = PLATFORM.to_string();
                output.terminal = TERMINAL.to_string();
                output
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises the registered Host, real PTY and Xcode lookup in both native shells.
    #[test]
    fn unsandbox_registers_native_login_shells_with_controlling_tty() {
        if crate::sandbox::processIsolation() != operit_host_api::HostIsolation::None {
            return;
        }
        let host = AppleTerminalHost::new();
        let info = host.terminalInfo().unwrap();
        assert!(info.types.iter().any(|kind| kind.terminalType == "bash"));
        assert!(info.types.iter().any(|kind| kind.terminalType == "zsh"));
        for kind in &info.types {
            let session = host
                .createOrGetTypedSession("native-login-test", &kind.terminalType)
                .unwrap();
            let result = host.executeInSession(
                &session.sessionId,
                "test -t 0 && test -t 1 && ( : </dev/tty ) && /usr/bin/xcrun --find clang && printf 'operit-native-pty-ok\\n'",
                10000,
            ).unwrap();
            host.closeSession(&session.sessionId).unwrap();
            assert!(!result.timedOut, "{} timed out", kind.terminalType);
            assert_eq!(
                result.exitCode, 0,
                "{}: {}",
                kind.terminalType, result.output
            );
            assert!(result.output.contains("operit-native-pty-ok"));
        }
        assert!(host.listSessions().unwrap().is_empty());
    }
}

#[cfg(test)]
mod exit_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn exited_native_shell_remains_pollable_and_closable() {
        if crate::sandbox::processIsolation() != operit_host_api::HostIsolation::None {
            return;
        }
        let host = AppleTerminalHost::new();
        let session = host.createOrGetSession("native-exit-test").unwrap();
        host.writePtySession(&session.sessionId, b"exit 7\r")
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(code) = host.pollPtyExitCode(&session.sessionId).unwrap() {
                assert_eq!(code, 7);
                break;
            }
            assert!(Instant::now() < deadline, "native shell did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(host.listSessions().unwrap().is_empty());
        host.getSessionScreen(&session.sessionId).unwrap();
        host.closeSession(&session.sessionId).unwrap();
    }
}
