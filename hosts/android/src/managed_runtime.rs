use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use operit_host_api::{
    HostError, HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest,
};

use crate::runtime_common::{
    buildAndroidProotCommand, requiredAndroidRuntimePath, validateRootfsExecutable,
};

const MANAGED_RUNTIME_STDIO_BUFFER_BYTES: usize = 64 * 1024;
const MANAGED_RUNTIME_SINGLE_FRAME_MIN_BYTES: usize = 4 * 1024;

#[derive(Clone, Default)]
pub struct AndroidManagedRuntimeHost;

impl AndroidManagedRuntimeHost {
    /// Creates an Android managed runtime host.
    pub fn new() -> Self {
        Self
    }
}

struct AndroidManagedRuntimeProcess {
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    stdoutRx: Mutex<Receiver<String>>,
    stderrLines: Arc<Mutex<VecDeque<String>>>,
}

impl ManagedRuntimeProcess for AndroidManagedRuntimeProcess {
    /// Writes one protocol line to the managed runtime stdin.
    fn writeLine(&self, line: &str) -> HostResult<()> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| HostError::new("stdin mutex poisoned"))?;
        writeManagedRuntimeLine(&mut stdin, line)
    }

    /// Writes multiple protocol lines to the managed runtime stdin.
    fn writeLines(&self, lines: &[String]) -> HostResult<()> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| HostError::new("stdin mutex poisoned"))?;
        writeManagedRuntimeLines(&mut stdin, lines)
    }

    /// Reads one protocol line from the managed runtime stdout queue.
    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>> {
        let receiver = self
            .stdoutRx
            .lock()
            .map_err(|_| HostError::new("stdout mutex poisoned"))?;
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
            .map_err(|_| HostError::new("stderr mutex poisoned"))?;
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
            .map_err(|_| HostError::new("child mutex poisoned"))?;
        Ok(child.try_wait()?.is_none())
    }

    /// Terminates the managed runtime process.
    fn kill(&self) -> HostResult<()> {
        let mut child = self
            .child
            .lock()
            .map_err(|_| HostError::new("child mutex poisoned"))?;
        match child.try_wait()? {
            Some(_) => Ok(()),
            None => {
                child.kill()?;
                Ok(())
            }
        }
    }
}

impl ManagedRuntimeHost for AndroidManagedRuntimeHost {
    /// Returns the persistent Android managed runtime workspace directory.
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
        let dir = internalRoot.join("managed_runtime");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.to_string_lossy().to_string())
    }

    /// Resolves a managed runtime executable inside the Android rootfs.
    fn resolveRuntimeExecutable(
        &self,
        program: ManagedRuntimeProgram,
        executablePath: Option<&str>,
    ) -> HostResult<String> {
        let executable = match executablePath.map(str::trim) {
            Some(value) if !value.is_empty() => value.to_string(),
            _ => match program {
                ManagedRuntimeProgram::Node => "/usr/bin/node".to_string(),
                ManagedRuntimeProgram::Python => "/usr/bin/python3".to_string(),
                ManagedRuntimeProgram::Uv => "/usr/bin/uv".to_string(),
                ManagedRuntimeProgram::Pnpm => "/usr/bin/pnpm".to_string(),
            },
        };
        validateRootfsExecutable(&executable)?;
        Ok(executable)
    }

    /// Starts a persistent Android proot managed runtime process with piped stdio.
    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        let executable = self
            .resolveRuntimeExecutable(request.program.clone(), request.executablePath.as_deref())?;
        let mut command = buildAndroidProotCommand(&executable, request.cwd.as_deref())?;
        command.args(request.args);
        command.envs(request.env);
        command.env("PROOT_NO_SECCOMP", "1");
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::new("managed runtime process has no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HostError::new("managed runtime process has no stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| HostError::new("managed runtime process has no stderr"))?;

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

        Ok(Box::new(AndroidManagedRuntimeProcess {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdoutRx: Mutex::new(stdoutRx),
            stderrLines,
        }))
    }

    /// Runs a one-shot Android proot managed runtime command and captures output.
    fn runRuntimeCommand(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<RuntimeCommandOutput> {
        let executable = self
            .resolveRuntimeExecutable(request.program.clone(), request.executablePath.as_deref())?;
        let mut command = buildAndroidProotCommand(&executable, request.cwd.as_deref())?;
        command.args(request.args);
        command.envs(request.env);
        command.env("PROOT_NO_SECCOMP", "1");
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
