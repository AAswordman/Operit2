use std::path::Path;
use std::sync::Arc;

use operit_host_api::{
    HostError, HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest,
};
use operit_host_native_common::{TerminalManagedRuntimeLaunch, TerminalManagedRuntimeProcess};

use crate::terminal::IosTerminalHost;

const ISH_TERMINAL: &str = "ish";
const SHELL_TERMINAL_TYPE: &str = "shell";
const ISH_RUNTIME_WORKSPACE: &str = "/root/.operit/managed_runtime";

/// Starts managed processes inside the persistent iSH Alpine environment.
#[derive(Clone)]
pub struct IosManagedRuntimeHost {
    terminalHost: Arc<IosTerminalHost>,
}

impl IosManagedRuntimeHost {
    /// Creates an iOS managed runtime host sharing the embedded iSH terminal owner.
    pub fn new(terminalHost: Arc<IosTerminalHost>) -> Self {
        Self { terminalHost }
    }

    /// Builds the iSH Alpine launch description for one managed runtime request.
    fn buildLaunch(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<TerminalManagedRuntimeLaunch> {
        let RuntimeProcessRequest {
            program,
            executablePath,
            args,
            cwd,
            env,
        } = request;
        let program = self.resolveRuntimeExecutable(program, executablePath.as_deref())?;
        let (
            sessionWorkingDirectory,
            processWorkingDirectory,
            ensureProcessWorkingDirectory,
            program,
            args,
            env,
        ) = match cwd {
            Some(hostWorkingDirectory) => {
                let runtimeWorkingDirectory =
                    self.mountRuntimeWorkingDirectory(&hostWorkingDirectory)?;
                (
                    runtimeWorkingDirectory.clone(),
                    runtimeWorkingDirectory,
                    false,
                    program,
                    args,
                    env,
                )
            }
            None => (
                "/root".to_string(),
                ISH_RUNTIME_WORKSPACE.to_string(),
                true,
                program,
                args,
                env,
            ),
        };
        Ok(TerminalManagedRuntimeLaunch {
            terminal: ISH_TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            sessionWorkingDirectory,
            processWorkingDirectory,
            ensureProcessWorkingDirectory,
            program,
            args,
            env,
        })
    }

    /// Shares the host parent at its absolute path; arguments and environment
    /// keep the same paths as the filesystem and workspace services.
    fn mountRuntimeWorkingDirectory(&self, hostWorkingDirectory: &str) -> HostResult<String> {
        let hostWorkingDirectory = Path::new(hostWorkingDirectory);
        if !hostWorkingDirectory.is_absolute() {
            return Err(HostError::new(format!(
                "iSH managed runtime working directory must be absolute: {}",
                hostWorkingDirectory.to_string_lossy()
            )));
        }
        let hostParent = hostWorkingDirectory.parent().ok_or_else(|| {
            HostError::new(format!(
                "iSH managed runtime working directory has no parent: {}",
                hostWorkingDirectory.to_string_lossy()
            ))
        })?;
        let hostParent = hostParent.to_str().ok_or_else(|| {
            HostError::new("iSH managed runtime parent directory is not valid UTF-8")
        })?;
        self.terminalHost
            .mountManagedRuntimeDirectory(hostParent, hostParent)?;
        Ok(hostWorkingDirectory.to_string_lossy().into_owned())
    }

    /// Starts one iSH managed runtime process for a request.
    fn startProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<TerminalManagedRuntimeProcess> {
        TerminalManagedRuntimeProcess::start(self.terminalHost.clone(), self.buildLaunch(request)?)
    }
}

impl ManagedRuntimeHost for IosManagedRuntimeHost {
    /// Returns the persistent runtime workspace located inside the iSH Alpine filesystem.
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        Ok(ISH_RUNTIME_WORKSPACE.to_string())
    }

    /// Resolves a runtime executable inside the embedded iSH Alpine filesystem.
    fn resolveRuntimeExecutable(
        &self,
        program: ManagedRuntimeProgram,
        executablePath: Option<&str>,
    ) -> HostResult<String> {
        if let Some(path) = executablePath {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
        Ok(match program {
            ManagedRuntimeProgram::Node => "/usr/bin/node".to_string(),
            ManagedRuntimeProgram::Python => "/usr/bin/python3".to_string(),
            ManagedRuntimeProgram::Uv => "/usr/bin/uv".to_string(),
            ManagedRuntimeProgram::Pnpm => "/usr/bin/pnpm".to_string(),
        })
    }

    /// Starts a persistent iSH Alpine process.
    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        Ok(Box::new(self.startProcess(request)?))
    }

    /// Runs a one-shot command inside iSH Alpine and captures its terminal output.
    fn runRuntimeCommand(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<RuntimeCommandOutput> {
        let process = self.startProcess(request)?;
        let exitCode = process.waitForExit()?;
        let stdout = process.takeOutputText()?;
        Ok(RuntimeCommandOutput {
            exitCode: Some(exitCode),
            stdout,
            stderr: String::new(),
        })
    }
}
