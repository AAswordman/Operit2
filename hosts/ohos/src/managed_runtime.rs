use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use operit_host_api::{
    HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest, TerminalHost,
};
use operit_host_native_common::{TerminalManagedRuntimeLaunch, TerminalManagedRuntimeProcess};

use crate::terminal::{OhosTerminalHost, VROOT_HOST_MOUNT_ROOT};

const QEMU_VROOT_TERMINAL: &str = "qemu-vroot";
const SHELL_TERMINAL_TYPE: &str = "shell";

/// Starts OpenHarmony MCP runtimes inside the packaged QEMU-vroot Alpine environment.
#[derive(Clone)]
pub struct OhosManagedRuntimeHost {
    terminalHost: Arc<dyn TerminalHost>,
    workspaceRoot: PathBuf,
}

impl OhosManagedRuntimeHost {
    /// Creates an OpenHarmony managed runtime host sharing the QEMU-vroot terminal owner.
    pub fn new(terminalHost: Arc<OhosTerminalHost>, workspaceRoot: PathBuf) -> Self {
        Self {
            terminalHost,
            workspaceRoot,
        }
    }

    /// Builds the packaged-Alpine launch description for one managed runtime request.
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
        let hostWorkingDirectory =
            cwd.unwrap_or_else(|| self.workspaceRoot.to_string_lossy().to_string());
        let processWorkingDirectory = vrootHostPath(&hostWorkingDirectory)?;
        let program = mapVrootPath(&program, &hostWorkingDirectory, &processWorkingDirectory);
        let args = args
            .iter()
            .map(|arg| mapVrootPath(arg, &hostWorkingDirectory, &processWorkingDirectory))
            .collect();
        let env = mapVrootEnvironment(env, &hostWorkingDirectory, &processWorkingDirectory);
        Ok(TerminalManagedRuntimeLaunch {
            terminal: QEMU_VROOT_TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            sessionWorkingDirectory: hostWorkingDirectory,
            processWorkingDirectory,
            ensureProcessWorkingDirectory: false,
            program,
            args,
            env,
        })
    }

    /// Starts one QEMU-vroot managed runtime process for a request.
    fn startProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<TerminalManagedRuntimeProcess> {
        TerminalManagedRuntimeProcess::start(self.terminalHost.clone(), self.buildLaunch(request)?)
    }
}

/// Converts one absolute OpenHarmony host path into its QEMU-vroot mounted path.
fn vrootHostPath(hostPath: &str) -> HostResult<String> {
    let hostPath = Path::new(hostPath);
    if !hostPath.is_absolute() {
        return Err(operit_host_api::HostError::new(format!(
            "OpenHarmony MCP working directory must be absolute: {}",
            hostPath.to_string_lossy()
        )));
    }
    let hostPath = hostPath.to_str().ok_or_else(|| {
        operit_host_api::HostError::new("OpenHarmony MCP working directory is not valid UTF-8")
    })?;
    Ok(format!("{VROOT_HOST_MOUNT_ROOT}{hostPath}"))
}

/// Maps one path rooted in the MCP host directory into the QEMU-vroot mount.
fn mapVrootPath(value: &str, hostWorkingDirectory: &str, vrootWorkingDirectory: &str) -> String {
    let hostWorkingDirectory = hostWorkingDirectory.trim_end_matches('/');
    match value.strip_prefix(hostWorkingDirectory) {
        Some(suffix) if suffix.is_empty() || suffix.starts_with('/') => {
            format!("{vrootWorkingDirectory}{suffix}")
        }
        _ => value.to_string(),
    }
}

/// Maps MCP environment values rooted in the host plugin directory into QEMU-vroot paths.
fn mapVrootEnvironment(
    mut environment: BTreeMap<String, String>,
    hostWorkingDirectory: &str,
    vrootWorkingDirectory: &str,
) -> BTreeMap<String, String> {
    for value in environment.values_mut() {
        *value = mapVrootPath(value, hostWorkingDirectory, vrootWorkingDirectory);
    }
    environment
}

impl ManagedRuntimeHost for OhosManagedRuntimeHost {
    /// Returns the persistent OpenHarmony workspace mounted into QEMU-vroot.
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let directory = self.workspaceRoot.join(".operit").join("managed_runtime");
        std::fs::create_dir_all(&directory)?;
        Ok(directory.to_string_lossy().to_string())
    }

    /// Resolves a runtime executable inside the packaged Alpine root filesystem.
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

    /// Starts a persistent OpenHarmony MCP process inside QEMU-vroot Alpine.
    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        Ok(Box::new(self.startProcess(request)?))
    }

    /// Runs a one-shot command inside QEMU-vroot Alpine and captures its terminal output.
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
