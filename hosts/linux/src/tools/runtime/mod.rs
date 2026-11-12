use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
pub struct LinuxManagedRuntimeHost;

impl LinuxManagedRuntimeHost {
    /// Creates a Linux managed runtime host.
    pub fn new() -> Self {
        Self
    }
}

impl ManagedRuntimeHost for LinuxManagedRuntimeHost {
    /// Launches explicit native path intents with argument vectors and no UI-layer platform checks.
    fn openPath(&self, path: &str, action: RuntimePathAction) -> HostResult<()> {
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
                let mut command = Command::new("xdg-open");
                command.arg(directory);
                command
            }
            RuntimePathAction::Terminal => {
                let mut command = Command::new("x-terminal-emulator");
                command.current_dir(directory);
                command
            }
            RuntimePathAction::Application { program, args } => {
                let executable =
                    self.resolveRuntimeExecutable(ManagedRuntimeProgram::Command(program), None)?;
                let mut command = Command::new(executable);
                command.args(args).arg(path).current_dir(directory);
                command
            }
        };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    /// Returns the persistent Linux managed runtime workspace directory.
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
            ManagedRuntimeProgram::Python => vec!["python3"],
            ManagedRuntimeProgram::Uv => vec!["uv"],
            ManagedRuntimeProgram::Pnpm => vec!["pnpm"],
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
