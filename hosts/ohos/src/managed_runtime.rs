use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use operit_host_api::{
    HostError, HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest,
};

const MANAGED_RUNTIME_STDIO_BUFFER_BYTES: usize = 64 * 1024;
const MANAGED_RUNTIME_SINGLE_FRAME_MIN_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug)]
pub struct OhosManagedRuntimeHost {
    workspaceRoot: PathBuf,
}

impl OhosManagedRuntimeHost {
    /// Creates an OpenHarmony managed runtime host rooted in app workspace storage.
    pub fn new(workspaceRoot: PathBuf) -> Self {
        Self { workspaceRoot }
    }
}

struct OhosManagedRuntimeProcess {
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    stdoutRx: Mutex<Receiver<String>>,
    stderrLines: Arc<Mutex<VecDeque<String>>>,
}

impl ManagedRuntimeProcess for OhosManagedRuntimeProcess {
    /// Writes one protocol line to the managed runtime stdin.
    fn writeLine(&self, line: &str) -> HostResult<()> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| HostError::new("OpenHarmony managed runtime stdin mutex poisoned"))?;
        writeManagedRuntimeLine(&mut stdin, line)
    }

    /// Writes multiple protocol lines to the managed runtime stdin.
    fn writeLines(&self, lines: &[String]) -> HostResult<()> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| HostError::new("OpenHarmony managed runtime stdin mutex poisoned"))?;
        writeManagedRuntimeLines(&mut stdin, lines)
    }

    /// Reads one protocol line from the managed runtime stdout queue.
    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>> {
        let receiver = self
            .stdoutRx
            .lock()
            .map_err(|_| HostError::new("OpenHarmony managed runtime stdout mutex poisoned"))?;
        match receiver.recv_timeout(Duration::from_millis(timeoutMs)) {
            Ok(line) => Ok(Some(line)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(None),
        }
    }

    /// Drains buffered stderr lines collected from the managed runtime.
    fn drainStderr(&self) -> HostResult<String> {
        let mut lines = self
            .stderrLines
            .lock()
            .map_err(|_| HostError::new("OpenHarmony managed runtime stderr mutex poisoned"))?;
        let mut output = String::new();
        while let Some(line) = lines.pop_front() {
            output.push_str(&line);
            if !line.ends_with('\n') {
                output.push('\n');
            }
        }
        Ok(output)
    }

    /// Returns whether the managed runtime process is still alive.
    fn isRunning(&self) -> HostResult<bool> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| HostError::new("OpenHarmony managed runtime child mutex poisoned"))?;
        Ok(child.try_wait()?.is_none())
    }

    /// Terminates the managed runtime process.
    fn kill(&self) -> HostResult<()> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| HostError::new("OpenHarmony managed runtime child mutex poisoned"))?;
        match child.try_wait()? {
            Some(_) => Ok(()),
            None => {
                child.kill()?;
                Ok(())
            }
        }
    }
}

impl ManagedRuntimeHost for OhosManagedRuntimeHost {
    /// Returns the persistent OpenHarmony managed runtime workspace directory.
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let dir = self.workspaceRoot.join(".operit").join("managed_runtime");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.to_string_lossy().to_string())
    }

    /// Resolves a managed runtime executable from explicit path or PATH.
    fn resolveRuntimeExecutable(
        &self,
        program: ManagedRuntimeProgram,
        executablePath: Option<&str>,
    ) -> HostResult<String> {
        if let Some(path) = executablePath {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return ensureExecutablePath(trimmed);
            }
        }
        let names = match program {
            ManagedRuntimeProgram::Node => vec!["node"],
            ManagedRuntimeProgram::Python => vec!["python3", "python"],
            ManagedRuntimeProgram::Uv => vec!["uv"],
            ManagedRuntimeProgram::Pnpm => vec!["pnpm"],
        };
        findExecutable(&names).ok_or_else(|| {
            HostError::new(format!(
                "OpenHarmony managed runtime executable not found for {:?}",
                program
            ))
        })
    }

    /// Starts a persistent managed runtime subprocess with piped stdio.
    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        let executable = self
            .resolveRuntimeExecutable(request.program.clone(), request.executablePath.as_deref())?;
        let mut command = Command::new(executable);
        command.args(request.args);
        if let Some(cwd) = request.cwd {
            command.current_dir(cwd);
        }
        command.envs(request.env);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::new("OpenHarmony managed runtime process has no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HostError::new("OpenHarmony managed runtime process has no stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| HostError::new("OpenHarmony managed runtime process has no stderr"))?;
        let (stdoutTx, stdoutRx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::with_capacity(MANAGED_RUNTIME_STDIO_BUFFER_BYTES, stdout)
                .lines()
                .flatten()
            {
                let _ = stdoutTx.send(line);
            }
        });
        let stderrLines = Arc::new(Mutex::new(VecDeque::new()));
        let stderrLinesForThread = stderrLines.clone();
        thread::spawn(move || {
            for line in BufReader::with_capacity(MANAGED_RUNTIME_STDIO_BUFFER_BYTES, stderr)
                .lines()
                .flatten()
            {
                if let Ok(mut lines) = stderrLinesForThread.lock() {
                    lines.push_back(line);
                    while lines.len() > 400 {
                        lines.pop_front();
                    }
                }
            }
        });
        Ok(Box::new(OhosManagedRuntimeProcess {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdoutRx: Mutex::new(stdoutRx),
            stderrLines,
        }))
    }

    /// Runs a one-shot managed runtime command and captures output.
    fn runRuntimeCommand(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<RuntimeCommandOutput> {
        let executable = self
            .resolveRuntimeExecutable(request.program.clone(), request.executablePath.as_deref())?;
        let mut command = Command::new(executable);
        command.args(request.args);
        if let Some(cwd) = request.cwd {
            command.current_dir(cwd);
        }
        command.envs(request.env);
        let output = command.output()?;
        Ok(RuntimeCommandOutput {
            exitCode: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

/// Writes one newline-terminated managed runtime frame.
#[allow(non_snake_case)]
fn writeManagedRuntimeLine(stdin: &mut ChildStdin, line: &str) -> HostResult<()> {
    let lineBytes = line.as_bytes();
    match lineBytes.len() >= MANAGED_RUNTIME_SINGLE_FRAME_MIN_BYTES {
        true => writeManagedRuntimeLargeLine(stdin, lineBytes),
        false => writeManagedRuntimeSmallLine(stdin, lineBytes),
    }
}

/// Writes a small managed runtime line without per-message heap allocation.
#[allow(non_snake_case)]
fn writeManagedRuntimeSmallLine(stdin: &mut ChildStdin, lineBytes: &[u8]) -> HostResult<()> {
    stdin.write_all(lineBytes)?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;
    Ok(())
}

/// Writes a large managed runtime line as one contiguous pipe frame.
#[allow(non_snake_case)]
fn writeManagedRuntimeLargeLine(stdin: &mut ChildStdin, lineBytes: &[u8]) -> HostResult<()> {
    let mut frame = Vec::with_capacity(lineBytes.len() + 1);
    frame.extend_from_slice(lineBytes);
    frame.push(b'\n');
    stdin.write_all(&frame)?;
    stdin.flush()?;
    Ok(())
}

/// Writes many managed runtime lines through one contiguous pipe frame.
#[allow(non_snake_case)]
fn writeManagedRuntimeLines(stdin: &mut ChildStdin, lines: &[String]) -> HostResult<()> {
    let frameBytes = lines.iter().map(|line| line.len() + 1).sum();
    let mut frame = Vec::with_capacity(frameBytes);
    for line in lines {
        frame.extend_from_slice(line.as_bytes());
        frame.push(b'\n');
    }
    stdin.write_all(&frame)?;
    stdin.flush()?;
    Ok(())
}

/// Resolves a directly supplied executable path.
#[allow(non_snake_case)]
fn ensureExecutablePath(path: &str) -> HostResult<String> {
    let candidate = PathBuf::from(path);
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }
    findExecutable(&[path]).ok_or_else(|| HostError::new(format!("Executable not found: {path}")))
}

/// Finds the first executable candidate from PATH.
#[allow(non_snake_case)]
fn findExecutable(names: &[&str]) -> Option<String> {
    let pathValue = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&pathValue) {
        for name in names {
            let candidate = dir.join(name);
            if isExecutableCandidate(&candidate) {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// Returns whether a path points to an executable file candidate.
#[allow(non_snake_case)]
fn isExecutableCandidate(path: &Path) -> bool {
    path.is_file()
}
