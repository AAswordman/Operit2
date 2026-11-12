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
pub struct AppleManagedRuntimeHost;

impl AppleManagedRuntimeHost {
    /// Creates an Apple managed runtime host.
    pub fn new() -> Self {
        Self
    }
}

impl ManagedRuntimeHost for AppleManagedRuntimeHost {
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
                let mut command = Command::new("open");
                command.arg("-R").arg(path);
                command
            }
            RuntimePathAction::Terminal => {
                let mut command = Command::new("open");
                command.args(["-a", "Terminal"]).arg(directory);
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

    /// Returns the persistent Apple managed runtime workspace directory.
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let dir = appleApplicationSupportDirectory()
            .join("Operit2")
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
            ManagedRuntimeProgram::Python => vec!["python3", "python"],
            ManagedRuntimeProgram::Uv => vec!["uv"],
            ManagedRuntimeProgram::Pnpm => vec!["pnpm"],
            ManagedRuntimeProgram::Command(ref name) => vec![name.as_str()],
        };
        match findExecutable(&names) {
            Some(executable) => Ok(executable),
            None => Err(HostError::new(format!(
                "Managed runtime executable not found for {:?}",
                program
            ))),
        }
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

/// Returns the Apple application support directory for the current user.
fn appleApplicationSupportDirectory() -> PathBuf {
    let home = env::var_os("HOME").expect("HOME is required for Apple managed runtime storage");
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
}
