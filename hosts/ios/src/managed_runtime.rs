use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use operit_host_api::{
    HostError, HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest,
};
use operit_host_native_common::{TerminalManagedRuntimeLaunch, TerminalManagedRuntimeProcess};
use sha2::{Digest, Sha256};

use crate::terminal::IosTerminalHost;

const ISH_TERMINAL: &str = "ish";
const SHELL_TERMINAL_TYPE: &str = "shell";
const ISH_RUNTIME_WORKSPACE: &str = "/root/.operit/managed_runtime";

/// Starts iOS MCP runtimes inside the embedded iSH Alpine environment.
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
                let program =
                    mapRuntimePath(&program, &hostWorkingDirectory, &runtimeWorkingDirectory);
                let args = args
                    .iter()
                    .map(|arg| mapRuntimePath(arg, &hostWorkingDirectory, &runtimeWorkingDirectory))
                    .collect();
                let env =
                    mapRuntimeEnvironment(env, &hostWorkingDirectory, &runtimeWorkingDirectory);
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

    /// Mounts the App-owned MCP runtime parent and returns its iSH working-directory path.
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
        let directoryName = hostWorkingDirectory.file_name().ok_or_else(|| {
            HostError::new(format!(
                "iSH managed runtime working directory has no final component: {}",
                hostWorkingDirectory.to_string_lossy()
            ))
        })?;
        let hostParent = hostParent.to_str().ok_or_else(|| {
            HostError::new("iSH managed runtime parent directory is not valid UTF-8")
        })?;
        let directoryName = directoryName.to_str().ok_or_else(|| {
            HostError::new("iSH managed runtime directory name is not valid UTF-8")
        })?;
        let mountPoint = runtimeMountPoint(hostParent);
        self.terminalHost
            .mountManagedRuntimeDirectory(hostParent, &mountPoint)?;
        Ok(format!("{mountPoint}/{directoryName}"))
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

    /// Starts a persistent iSH Alpine MCP process.
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

/// Derives the stable iSH mount point for one App-owned runtime directory parent.
fn runtimeMountPoint(hostParent: &str) -> String {
    let digest = Sha256::digest(hostParent.as_bytes());
    let suffix = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("/mnt/operit-mcp/{suffix}")
}

/// Maps one exact host-runtime path into the corresponding iSH-mounted path.
fn mapRuntimePath(
    value: &str,
    hostWorkingDirectory: &str,
    runtimeWorkingDirectory: &str,
) -> String {
    let normalizedHostDirectory = hostWorkingDirectory.trim_end_matches('/');
    match value.strip_prefix(normalizedHostDirectory) {
        Some(suffix) if suffix.is_empty() || suffix.starts_with('/') => {
            format!("{runtimeWorkingDirectory}{suffix}")
        }
        _ => value.to_string(),
    }
}

/// Maps MCP environment values rooted at the host plugin directory into the iSH mounted path.
fn mapRuntimeEnvironment(
    mut environment: BTreeMap<String, String>,
    hostWorkingDirectory: &str,
    runtimeWorkingDirectory: &str,
) -> BTreeMap<String, String> {
    for value in environment.values_mut() {
        *value = mapRuntimePath(value, hostWorkingDirectory, runtimeWorkingDirectory);
    }
    environment
}
