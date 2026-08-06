use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use operit_host_api::{HostError, HostResult, ManagedRuntimeProcess, TerminalHost};

const MANAGED_RUNTIME_ROWS: u16 = 24;
const MANAGED_RUNTIME_COLUMNS: u16 = 80;
const MANAGED_RUNTIME_POLL_INTERVAL: Duration = Duration::from_millis(10);
const MANAGED_RUNTIME_LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);
const MANAGED_RUNTIME_OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
static NEXT_MANAGED_RUNTIME_SESSION: AtomicU64 = AtomicU64::new(1);

/// Describes one managed-runtime program launched through an exact terminal implementation.
pub struct TerminalManagedRuntimeLaunch {
    pub terminal: String,
    pub terminalType: String,
    pub sessionWorkingDirectory: String,
    pub processWorkingDirectory: String,
    pub ensureProcessWorkingDirectory: bool,
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

/// Adapts a terminal PTY session into the line-oriented managed-runtime process contract.
pub struct TerminalManagedRuntimeProcess {
    terminalHost: Arc<dyn TerminalHost>,
    sessionId: String,
    output: Mutex<VecDeque<u8>>,
    closed: AtomicBool,
}

impl TerminalManagedRuntimeProcess {
    /// Starts one program after the terminal shell has disabled echo and replaced itself with it.
    pub fn start(
        terminalHost: Arc<dyn TerminalHost>,
        launch: TerminalManagedRuntimeLaunch,
    ) -> HostResult<Self> {
        let sessionName = format!(
            "managed-runtime-{}",
            NEXT_MANAGED_RUNTIME_SESSION.fetch_add(1, Ordering::Relaxed)
        );
        let sessionId = terminalHost.startPtySession(
            &sessionName,
            &launch.terminal,
            &launch.terminalType,
            &launch.sessionWorkingDirectory,
            MANAGED_RUNTIME_ROWS,
            MANAGED_RUNTIME_COLUMNS,
        )?;
        let readyMarker = format!(
            "OPERIT_MANAGED_RUNTIME_READY_{}",
            NEXT_MANAGED_RUNTIME_SESSION.fetch_add(1, Ordering::Relaxed)
        );
        let command = buildRuntimeLaunchCommand(&launch, &readyMarker)?;
        if let Err(error) = terminalHost.writePtySession(&sessionId, command.as_bytes()) {
            let _ = terminalHost.closePtySession(&sessionId);
            return Err(error);
        }
        let process = Self {
            terminalHost,
            sessionId,
            output: Mutex::new(VecDeque::new()),
            closed: AtomicBool::new(false),
        };
        if let Err(error) = process.waitForLaunchReady(&readyMarker) {
            let _ = process.closeSession();
            return Err(error);
        }
        Ok(process)
    }

    /// Waits for the terminal-owned program to exit and returns its exact exit code.
    pub fn waitForExit(&self) -> HostResult<i32> {
        loop {
            self.collectOutput()?;
            if let Some(exitCode) = self.terminalHost.pollPtyExitCode(&self.sessionId)? {
                self.collectOutput()?;
                return Ok(exitCode);
            }
            thread::sleep(MANAGED_RUNTIME_POLL_INTERVAL);
        }
    }

    /// Returns all terminal output accumulated for a completed one-shot command.
    pub fn takeOutputText(&self) -> HostResult<String> {
        self.collectOutput()?;
        let mut output = self
            .output
            .lock()
            .map_err(|_| HostError::new("managed runtime terminal output mutex poisoned"))?;
        let bytes = output.drain(..).collect::<Vec<_>>();
        String::from_utf8(bytes).map_err(|error| {
            HostError::new(format!("managed runtime output is not UTF-8: {error}"))
        })
    }

    /// Reads any bytes currently available from the terminal into the process output buffer.
    fn collectOutput(&self) -> HostResult<()> {
        let incoming = self.terminalHost.readPtySession(&self.sessionId)?;
        if incoming.is_empty() {
            return Ok(());
        }
        let mut output = self
            .output
            .lock()
            .map_err(|_| HostError::new("managed runtime terminal output mutex poisoned"))?;
        if output.len() + incoming.len() > MANAGED_RUNTIME_OUTPUT_LIMIT {
            return Err(HostError::new(format!(
                "managed runtime terminal output exceeded {MANAGED_RUNTIME_OUTPUT_LIMIT} bytes"
            )));
        }
        output.extend(incoming);
        Ok(())
    }

    /// Waits until the shell has disabled echo and is ready to accept MCP protocol input.
    fn waitForLaunchReady(&self, readyMarker: &str) -> HostResult<()> {
        let marker = format!("\u{1e}{readyMarker}\u{1f}\n").into_bytes();
        let deadline = Instant::now() + MANAGED_RUNTIME_LAUNCH_TIMEOUT;
        let mut output = Vec::new();
        loop {
            let incoming = self.terminalHost.readPtySession(&self.sessionId)?;
            if !incoming.is_empty() {
                if output.len() + incoming.len() > MANAGED_RUNTIME_OUTPUT_LIMIT {
                    return Err(HostError::new(format!(
                        "managed runtime terminal launch output exceeded {MANAGED_RUNTIME_OUTPUT_LIMIT} bytes"
                    )));
                }
                output.extend(incoming);
                if let Some(markerStart) = output
                    .windows(marker.len())
                    .position(|candidate| candidate == marker)
                {
                    let tailStart = markerStart + marker.len();
                    let mut bufferedOutput = self.output.lock().map_err(|_| {
                        HostError::new("managed runtime terminal output mutex poisoned")
                    })?;
                    bufferedOutput.extend(&output[tailStart..]);
                    return Ok(());
                }
            }
            if let Some(exitCode) = self.terminalHost.pollPtyExitCode(&self.sessionId)? {
                return Err(HostError::new(format!(
                    "managed runtime terminal exited before protocol readiness: {exitCode}"
                )));
            }
            if Instant::now() >= deadline {
                return Err(HostError::new(
                    "managed runtime terminal did not become protocol-ready within 30 seconds",
                ));
            }
            thread::sleep(MANAGED_RUNTIME_POLL_INTERVAL);
        }
    }

    /// Removes and returns one newline-terminated protocol line from buffered terminal output.
    fn takeBufferedLine(&self) -> HostResult<Option<String>> {
        let mut output = self
            .output
            .lock()
            .map_err(|_| HostError::new("managed runtime terminal output mutex poisoned"))?;
        let Some(lineEnd) = output.iter().position(|byte| *byte == b'\n') else {
            return Ok(None);
        };
        let mut line = output.drain(..=lineEnd).collect::<Vec<_>>();
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        String::from_utf8(line).map(Some).map_err(|error| {
            HostError::new(format!(
                "managed runtime protocol line is not UTF-8: {error}"
            ))
        })
    }

    /// Closes the terminal session exactly once.
    fn closeSession(&self) -> HostResult<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.terminalHost.closePtySession(&self.sessionId)
    }
}

impl ManagedRuntimeProcess for TerminalManagedRuntimeProcess {
    /// Writes one JSON-RPC protocol line to the process stdin.
    fn writeLine(&self, line: &str) -> HostResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(HostError::new("managed runtime terminal session is closed"));
        }
        let mut frame = String::with_capacity(line.len() + 1);
        frame.push_str(line);
        frame.push('\n');
        self.terminalHost
            .writePtySession(&self.sessionId, frame.as_bytes())
            .map(|_| ())
    }

    /// Writes several JSON-RPC protocol lines to the process stdin in order.
    fn writeLines(&self, lines: &[String]) -> HostResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(HostError::new("managed runtime terminal session is closed"));
        }
        let frameBytes = lines.iter().map(|line| line.len() + 1).sum();
        let mut frame = String::with_capacity(frameBytes);
        for line in lines {
            frame.push_str(line);
            frame.push('\n');
        }
        self.terminalHost
            .writePtySession(&self.sessionId, frame.as_bytes())
            .map(|_| ())
    }

    /// Reads one JSON-RPC protocol line before the supplied timeout elapses.
    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>> {
        let deadline = Instant::now() + Duration::from_millis(timeoutMs);
        loop {
            if let Some(line) = self.takeBufferedLine()? {
                return Ok(Some(line));
            }
            self.collectOutput()?;
            if let Some(line) = self.takeBufferedLine()? {
                return Ok(Some(line));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            thread::sleep(remaining.min(MANAGED_RUNTIME_POLL_INTERVAL));
        }
    }

    /// Returns no separate stderr stream because the PTY transports both terminal streams together.
    fn drainStderr(&self) -> HostResult<String> {
        Ok(String::new())
    }

    /// Reports whether the terminal-owned program is still active.
    fn isRunning(&self) -> HostResult<bool> {
        if self.closed.load(Ordering::SeqCst) {
            return Ok(false);
        }
        Ok(self
            .terminalHost
            .pollPtyExitCode(&self.sessionId)?
            .is_none())
    }

    /// Terminates the terminal-owned program by closing its PTY session.
    fn kill(&self) -> HostResult<()> {
        self.closeSession()
    }
}

impl Drop for TerminalManagedRuntimeProcess {
    /// Releases the terminal session when its managed-runtime owner is dropped.
    fn drop(&mut self) {
        let _ = self.closeSession();
    }
}

/// Builds the shell command that creates the runtime directory and execs the requested program.
fn buildRuntimeLaunchCommand(
    launch: &TerminalManagedRuntimeLaunch,
    readyMarker: &str,
) -> HostResult<String> {
    let program = requiredRuntimeText(&launch.program, "program")?;
    let workingDirectory =
        requiredRuntimeText(&launch.processWorkingDirectory, "working_directory")?;
    let readyMarker = requiredRuntimeText(readyMarker, "ready_marker")?;
    let mut command = String::from("stty -echo && printf '\\036%s\\037\\n' ");
    command.push_str(&shellQuote(&readyMarker));
    command.push_str(" && ");
    if launch.ensureProcessWorkingDirectory {
        command.push_str("mkdir -p -- ");
        command.push_str(&shellQuote(&workingDirectory));
        command.push_str(" && ");
    }
    command.push_str("cd -- ");
    command.push_str(&shellQuote(&workingDirectory));
    command.push_str(" && exec");
    if !launch.env.is_empty() {
        command.push_str(" env");
        for (name, value) in &launch.env {
            validateEnvironmentName(name)?;
            command.push(' ');
            command.push_str(&shellQuote(&format!("{name}={value}")));
        }
    }
    command.push(' ');
    command.push_str(&shellQuote(&program));
    for arg in &launch.args {
        command.push(' ');
        command.push_str(&shellQuote(arg));
    }
    command.push('\n');
    Ok(command)
}

/// Validates a required terminal-runtime text field.
fn requiredRuntimeText(value: &str, field: &str) -> HostResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HostError::new(format!(
            "managed runtime {field} must not be blank"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates one POSIX environment-variable name before it reaches the shell.
fn validateEnvironmentName(name: &str) -> HostResult<()> {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return Err(HostError::new(
            "managed runtime environment name must not be blank",
        ));
    };
    if !(first == '_' || first.is_ascii_alphabetic())
        || !characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(HostError::new(format!(
            "managed runtime environment name is invalid: {name}"
        )));
    }
    Ok(())
}

/// Quotes one exact string for a POSIX shell command argument.
fn shellQuote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
