use std::collections::VecDeque;
use std::env;
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

#[derive(Clone, Default)]
pub struct LinuxManagedRuntimeHost;

impl LinuxManagedRuntimeHost {
    pub fn new() -> Self {
        Self
    }
}

struct NativeManagedRuntimeProcess {
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    stdoutRx: Mutex<Receiver<String>>,
    stderrLines: Arc<Mutex<VecDeque<String>>>,
}

impl ManagedRuntimeProcess for NativeManagedRuntimeProcess {
    fn writeLine(&self, line: &str) -> HostResult<()> {
        let mut stdin = self.stdin.lock().map_err(|_| HostError::new("stdin mutex poisoned"))?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

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

    fn isRunning(&self) -> HostResult<bool> {
        let mut child = self.child.lock().map_err(|_| HostError::new("child mutex poisoned"))?;
        Ok(child.try_wait()?.is_none())
    }

    fn kill(&self) -> HostResult<()> {
        let mut child = self.child.lock().map_err(|_| HostError::new("child mutex poisoned"))?;
        match child.try_wait()? {
            Some(_) => Ok(()),
            None => {
                child.kill()?;
                Ok(())
            }
        }
    }
}

impl ManagedRuntimeHost for LinuxManagedRuntimeHost {
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let home = env::var_os("HOME")
            .ok_or_else(|| HostError::new("HOME is required for managed runtime storage"))?;
        let dir = PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("operit2")
            .join("managed_runtime");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.to_string_lossy().to_string())
    }

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
            ManagedRuntimeProgram::Python => vec!["python3"],
            ManagedRuntimeProgram::Uv => vec!["uv"],
            ManagedRuntimeProgram::Pnpm => vec!["pnpm"],
        };
        findExecutable(&names).ok_or_else(|| {
            HostError::new(format!(
                "Managed runtime executable not found for {:?}",
                program
            ))
        })
    }

    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        let executable =
            self.resolveRuntimeExecutable(request.program.clone(), request.executablePath.as_deref())?;
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
            for line in BufReader::new(stdout).lines().flatten() {
                let _ = stdoutTx.send(line);
            }
        });

        let stderrLines = Arc::new(Mutex::new(VecDeque::new()));
        let stderrLinesForThread = stderrLines.clone();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                if let Ok(mut lines) = stderrLinesForThread.lock() {
                    lines.push_back(line);
                    while lines.len() > 400 {
                        lines.pop_front();
                    }
                }
            }
        });

        Ok(Box::new(NativeManagedRuntimeProcess {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdoutRx: Mutex::new(stdoutRx),
            stderrLines,
        }))
    }

    fn runRuntimeCommand(&self, request: RuntimeProcessRequest) -> HostResult<RuntimeCommandOutput> {
        let executable =
            self.resolveRuntimeExecutable(request.program.clone(), request.executablePath.as_deref())?;
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

#[allow(non_snake_case)]
fn ensureExecutablePath(path: &str) -> HostResult<String> {
    let candidate = PathBuf::from(path);
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }
    findExecutable(&[path]).ok_or_else(|| HostError::new(format!("Executable not found: {path}")))
}

#[allow(non_snake_case)]
fn findExecutable(names: &[&str]) -> Option<String> {
    let pathValue = env::var_os("PATH")?;
    for dir in env::split_paths(&pathValue) {
        for name in names {
            let candidate = dir.join(name);
            if isExecutableCandidate(&candidate) {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

#[allow(non_snake_case)]
fn isExecutableCandidate(path: &Path) -> bool {
    path.is_file()
}
