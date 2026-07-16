use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use operit_host_api::{
    FileSystemHost, HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput,
    TerminalCommandOutput, TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput,
    TerminalSessionInfo, TerminalSessionListEntry, TerminalTypeInfo,
};

use crate::tools::fs::WebFileSystemHost;

#[derive(Clone)]
struct WebTerminalSession {
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
    screen: String,
    output: Vec<u8>,
}

thread_local! {
    static NEXT_TERMINAL_SESSION_ID: Cell<u64> = const { Cell::new(0) };
    static TERMINAL_SESSIONS: RefCell<BTreeMap<String, WebTerminalSession>> = const { RefCell::new(BTreeMap::new()) };
}

/// Simulates terminal sessions in browser memory.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebTerminalHost;

impl WebTerminalHost {
    /// Creates the browser terminal simulator host.
    pub fn new() -> Self {
        Self
    }
}

impl TerminalHost for WebTerminalHost {
    /// Describes the browser terminal simulator.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "web".to_string(),
            defaultType: "web-simulator".to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: "web-simulator".to_string(),
                available: true,
                description: "In-memory browser terminal simulator".to_string(),
            }],
        })
    }

    /// Creates an in-memory pseudo-terminal session.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let sessionId = nextTerminalSessionId();
        let banner = format!("Web microkernel booted\n{workingDir} $ ");
        let session = WebTerminalSession {
            sessionName: sessionName.to_string(),
            terminalType: terminalType.to_string(),
            workingDir: workingDir.to_string(),
            rows,
            cols,
            screen: banner.clone(),
            output: banner.into_bytes(),
        };
        TERMINAL_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(sessionId.clone(), session);
        });
        Ok(sessionId)
    }

    /// Drains unread pseudo-terminal output.
    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        TERMINAL_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let session = terminalSession(&mut sessions, sessionId)?;
            Ok(std::mem::take(&mut session.output))
        })
    }

    /// Appends terminal input to the simulated screen.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        TERMINAL_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let session = terminalSession(&mut sessions, sessionId)?;
            session.screen.push_str(&String::from_utf8_lossy(data));
            session.output.extend_from_slice(data);
            Ok(data.len())
        })
    }

    /// Updates the simulated pseudo-terminal dimensions.
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        TERMINAL_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let session = terminalSession(&mut sessions, sessionId)?;
            session.rows = rows;
            session.cols = cols;
            Ok(())
        })
    }

    /// Reports that simulated sessions remain open until explicitly closed.
    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        TERMINAL_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            terminalSession(&mut sessions, sessionId)?;
            Ok(None)
        })
    }

    /// Closes one simulated pseudo-terminal session.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        removeTerminalSession(sessionId)
    }

    /// Lists active simulated terminal sessions.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        TERMINAL_SESSIONS.with(|sessions| {
            Ok(sessions
                .borrow()
                .iter()
                .map(|(sessionId, session)| TerminalSessionListEntry {
                    sessionId: sessionId.clone(),
                    sessionName: session.sessionName.clone(),
                    terminalType: session.terminalType.clone(),
                    sessionKind: "web-simulator".to_string(),
                    workingDir: session.workingDir.clone(),
                    commandRunning: false,
                })
                .collect())
        })
    }

    /// Returns an existing named simulated terminal session or creates one.
    fn createOrGetSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let existing = TERMINAL_SESSIONS.with(|sessions| {
            sessions.borrow().iter().find_map(|(sessionId, session)| {
                (session.sessionName == sessionName).then(|| TerminalSessionInfo {
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
        let sessionId = self.startPtySession(sessionName, terminalType, "/", 24, 80)?;
        Ok(TerminalSessionInfo {
            sessionId,
            sessionName: sessionName.to_string(),
            terminalType: terminalType.to_string(),
            isNewSession: true,
        })
    }

    /// Runs one command through the browser microkernel.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        _timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        let (terminalType, output, exitCode) = runKernelCommand(sessionId, command)?;
        Ok(TerminalCommandOutput {
            command: command.to_string(),
            output,
            exitCode,
            sessionId: sessionId.to_string(),
            terminalType,
            timedOut: false,
        })
    }

    /// Reports that hidden operating-system commands are unavailable in browsers.
    fn executeHiddenCommand(
        &self,
        command: &str,
        terminalType: &str,
        executorKey: &str,
        _timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        Ok(HiddenTerminalCommandOutput {
            command: command.to_string(),
            output: "browser terminal simulator cannot execute operating-system commands"
                .to_string(),
            exitCode: 127,
            executorKey: executorKey.to_string(),
            terminalType: terminalType.to_string(),
            timedOut: false,
        })
    }

    /// Appends textual or control input to a simulated session.
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

    /// Closes one simulated terminal session and reports its result.
    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        removeTerminalSession(sessionId)?;
        Ok(TerminalCloseOutput {
            sessionId: sessionId.to_string(),
            success: true,
            message: "browser terminal simulator session closed".to_string(),
        })
    }

    /// Returns the current simulated terminal screen.
    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        TERMINAL_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let session = terminalSession(&mut sessions, sessionId)?;
            Ok(TerminalScreenOutput {
                sessionId: sessionId.to_string(),
                terminalType: session.terminalType.clone(),
                rows: session.rows as usize,
                cols: session.cols as usize,
                content: session.screen.clone(),
                commandRunning: false,
            })
        })
    }
}

/// Allocates one unique browser terminal session identifier.
#[allow(non_snake_case)]
fn nextTerminalSessionId() -> String {
    NEXT_TERMINAL_SESSION_ID.with(|next| {
        let value = next.get().saturating_add(1);
        next.set(value);
        format!("web-terminal-{value}")
    })
}

/// Returns one mutable simulated terminal session.
#[allow(non_snake_case)]
fn terminalSession<'a>(
    sessions: &'a mut BTreeMap<String, WebTerminalSession>,
    sessionId: &str,
) -> HostResult<&'a mut WebTerminalSession> {
    sessions
        .get_mut(sessionId)
        .ok_or_else(|| HostError::new(format!("browser terminal session not found: {sessionId}")))
}

/// Removes one simulated terminal session.
#[allow(non_snake_case)]
fn removeTerminalSession(sessionId: &str) -> HostResult<()> {
    TERMINAL_SESSIONS.with(|sessions| {
        sessions
            .borrow_mut()
            .remove(sessionId)
            .map(|_| ())
            .ok_or_else(|| {
                HostError::new(format!("browser terminal session not found: {sessionId}"))
            })
    })
}

/// Executes one browser microkernel command against the active terminal session.
#[allow(non_snake_case)]
fn runKernelCommand(sessionId: &str, command: &str) -> HostResult<(String, String, i32)> {
    TERMINAL_SESSIONS.with(|sessions| {
        let mut sessions = sessions.borrow_mut();
        let session = terminalSession(&mut sessions, sessionId)?;
        let (name, arguments) = splitKernelCommand(command);
        let outcome = executeKernelCommand(session, name, arguments);
        let (output, exitCode) = match outcome {
            Ok(output) => (output, 0),
            Err(error) => (error.message, 1),
        };
        let frame = format!(
            "\n{} $ {command}\n{output}\n{} $ ",
            session.workingDir, session.workingDir
        );
        session.screen.push_str(&frame);
        session.output.extend_from_slice(frame.as_bytes());
        Ok((session.terminalType.clone(), output, exitCode))
    })
}

/// Splits one microkernel command into its name and unparsed argument text.
#[allow(non_snake_case)]
fn splitKernelCommand(command: &str) -> (&str, &str) {
    match command.trim().split_once(char::is_whitespace) {
        Some((name, arguments)) => (name, arguments.trim()),
        None => (command.trim(), ""),
    }
}

/// Executes a supported browser microkernel command.
#[allow(non_snake_case)]
fn executeKernelCommand(
    session: &mut WebTerminalSession,
    name: &str,
    arguments: &str,
) -> HostResult<String> {
    match name {
        "" => Ok(String::new()),
        "help" => Ok("help pwd cd ls cat write mkdir rm echo js clear".to_string()),
        "pwd" => Ok(session.workingDir.clone()),
        "cd" => {
            session.workingDir = kernelPath(&session.workingDir, arguments)?;
            Ok(session.workingDir.clone())
        }
        "echo" => Ok(arguments.to_string()),
        "clear" => {
            session.screen.clear();
            Ok(String::new())
        }
        "ls" => {
            let path = kernelPath(&session.workingDir, arguments)?;
            WebFileSystemHost::new().listFiles(&path).map(|entries| {
                entries
                    .into_iter()
                    .map(|entry| entry.name)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        "cat" => {
            let path = kernelRequiredPath(&session.workingDir, arguments, "cat")?;
            WebFileSystemHost::new().readFile(&path)
        }
        "write" => {
            let (path, content) = splitKernelCommand(arguments);
            let path = kernelRequiredPath(&session.workingDir, path, "write")?;
            WebFileSystemHost::new().writeFile(&path, content, false)?;
            Ok(path)
        }
        "mkdir" => {
            let path = kernelRequiredPath(&session.workingDir, arguments, "mkdir")?;
            WebFileSystemHost::new().makeDirectory(&path, true)?;
            Ok(path)
        }
        "rm" => {
            let path = kernelRequiredPath(&session.workingDir, arguments, "rm")?;
            WebFileSystemHost::new().deleteFile(&path, true)?;
            Ok(path)
        }
        "js" => js_sys::eval(arguments)
            .map_err(|error| HostError::new(format!("microkernel JavaScript failed: {error:?}")))
            .map(|value| format!("{value:?}")),
        _ => Err(HostError::new(format!(
            "microkernel command not found: {name}"
        ))),
    }
}

/// Resolves a microkernel path against the terminal working directory.
#[allow(non_snake_case)]
fn kernelPath(workingDir: &str, argument: &str) -> HostResult<String> {
    let raw = if argument.is_empty() {
        workingDir
    } else {
        argument
    };
    let combined = if raw.starts_with('/') {
        raw.to_string()
    } else {
        format!("{}/{}", workingDir.trim_end_matches('/'), raw)
    };
    let mut segments = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            value => segments.push(value),
        }
    }
    Ok(format!("/{}", segments.join("/")))
}

/// Resolves a required command path or returns a command-specific error.
#[allow(non_snake_case)]
fn kernelRequiredPath(workingDir: &str, argument: &str, command: &str) -> HostResult<String> {
    if argument.is_empty() {
        return Err(HostError::new(format!(
            "microkernel {command} requires a path"
        )));
    }
    kernelPath(workingDir, argument)
}
