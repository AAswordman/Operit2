use std::process::Stdio;

use operit_host_api::{
    HostError, HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest,
};

use crate::runtime_common::{
    buildAndroidProotCommand, requiredAndroidRuntimePath, validateRootfsExecutable,
};

#[path = "../../common/managed_runtime_process.rs"]
mod managed_runtime_process;
use managed_runtime_process::NativeManagedRuntimeProcess;

#[derive(Clone, Default)]
pub struct AndroidManagedRuntimeHost;

impl AndroidManagedRuntimeHost {
    /// Creates an Android managed runtime host.
    pub fn new() -> Self {
        Self
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
                ManagedRuntimeProgram::Command(name) => name,
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

        NativeManagedRuntimeProcess::start(command.spawn()?)
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
        NativeManagedRuntimeProcess::run(command)
    }
}
