use crate::process::background_command;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use operit_host_api::{
    HostError, HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimePathAction, RuntimeProcessRequest,
};

#[path = "../../../../common/managed_runtime_process.rs"]
mod managed_runtime_process;
use managed_runtime_process::{
    ensureExecutablePath, findExecutable, runtimeCommand, NativeManagedRuntimeProcess,
};

#[derive(Clone, Default)]
pub struct WindowsManagedRuntimeHost;

impl WindowsManagedRuntimeHost {
    /// Creates a Windows managed runtime host.
    pub fn new() -> Self {
        Self
    }
}

impl ManagedRuntimeHost for WindowsManagedRuntimeHost {
    /// Launches external file, editor and terminal intents without constructing shell commands.
    fn openPath(&self, path: &str, action: RuntimePathAction) -> HostResult<()> {
        use std::os::windows::process::CommandExt;
        let is_terminal = matches!(&action, RuntimePathAction::Terminal);
        let target = Path::new(path);
        let directory = if target.is_dir() {
            target
        } else {
            target
                .parent()
                .ok_or_else(|| HostError::new("The target has no parent directory"))?
        };
        let mut command = match action {
            RuntimePathAction::Reveal => {
                let mut command = background_command("explorer.exe");
                if !target.is_dir() {
                    command.arg("/select,");
                }
                command.arg(path);
                command
            }
            RuntimePathAction::Terminal => {
                let mut command = std::process::Command::new("cmd.exe");
                command
                    .arg("/k")
                    .current_dir(directory)
                    .creation_flags(0x00000010);
                command
            }
            RuntimePathAction::Application { program, args } => {
                let executable =
                    self.resolveRuntimeExecutable(ManagedRuntimeProgram::Command(program), None)?;
                let mut command = background_command(executable);
                command.args(args).arg(path).current_dir(directory);
                command
            }
        };
        if is_terminal {
            command.spawn()?;
        } else {
            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
        }
        Ok(())
    }

    /// Returns the persistent Windows managed runtime workspace directory.
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let localAppData = env::var_os("LOCALAPPDATA").ok_or_else(|| {
            HostError::new("LOCALAPPDATA is required for managed runtime storage")
        })?;
        let dir = PathBuf::from(localAppData)
            .join("app.operit")
            .join("Operit2")
            .join("managed-runtime");
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
            ManagedRuntimeProgram::Node => vec!["node.exe"],
            ManagedRuntimeProgram::Python => vec!["python.exe"],
            ManagedRuntimeProgram::Uv => vec!["uv.exe"],
            ManagedRuntimeProgram::Pnpm => vec!["pnpm.cmd"],
            ManagedRuntimeProgram::Command(ref name) => vec![name.as_str()],
        };
        findExecutable(&names).ok_or_else(|| {
            HostError::new(format!(
                "Managed runtime executable not found for {:?}",
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
        let mut command = runtimeCommand(executable, request);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        NativeManagedRuntimeProcess::start(command.spawn()?)
    }

    /// Runs a one-shot managed runtime command and captures output.
    fn runRuntimeCommand(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<RuntimeCommandOutput> {
        let executable = self
            .resolveRuntimeExecutable(request.program.clone(), request.executablePath.as_deref())?;
        let command = runtimeCommand(executable, request);
        NativeManagedRuntimeProcess::run(command)
    }
}
