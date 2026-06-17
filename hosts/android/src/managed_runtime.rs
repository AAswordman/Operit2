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

#[derive(Clone, Default)]
pub struct AndroidManagedRuntimeHost;

impl AndroidManagedRuntimeHost {
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
    fn writeLine(&self, line: &str) -> HostResult<()> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| HostError::new("stdin mutex poisoned"))?;
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
        let mut child = self
            .child
            .lock()
            .map_err(|_| HostError::new("child mutex poisoned"))?;
        Ok(child.try_wait()?.is_none())
    }

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
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let storageRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_STORAGE_ROOT")?;
        let dir = storageRoot.join("managed_runtime");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.to_string_lossy().to_string())
    }

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

        Ok(Box::new(AndroidManagedRuntimeProcess {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdoutRx: Mutex::new(stdoutRx),
            stderrLines,
        }))
    }

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
