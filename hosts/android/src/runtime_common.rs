use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use operit_host_api::{HostError, HostResult};

#[allow(non_snake_case)]
pub(crate) fn requiredAndroidRuntimePath(name: &str) -> HostResult<PathBuf> {
    let path = env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| HostError::new(format!("{name} is required for Android managed runtime")))?;
    if path.exists() {
        Ok(path)
    } else {
        Err(HostError::new(format!(
            "Android managed runtime path does not exist: {}={}",
            name,
            path.to_string_lossy()
        )))
    }
}

#[allow(non_snake_case)]
pub(crate) fn validateRootfsExecutable(executable: &str) -> HostResult<()> {
    if !executable.starts_with('/') {
        return Ok(());
    }
    let rootfsDir = requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?;
    let candidate = rootfsDir.join(executable.trim_start_matches('/'));
    if candidate.is_file() {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "Android managed runtime executable does not exist in rootfs: {executable}"
        )))
    }
}

#[allow(non_snake_case)]
pub(crate) fn buildAndroidProotCommand(executable: &str, cwd: Option<&str>) -> HostResult<Command> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let rootfsDir = requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?;
    let storageRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_STORAGE_ROOT")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let proot = requiredAndroidRuntimePath("OPERIT_ANDROID_PROOT")?;
    let loader = requiredAndroidRuntimePath("OPERIT_ANDROID_LOADER")?;
    let _ = cwd;

    if !proot.is_file() {
        return Err(HostError::new(format!(
            "Android managed runtime proot does not exist: {}",
            proot.to_string_lossy()
        )));
    }
    if !loader.is_file() {
        return Err(HostError::new(format!(
            "Android managed runtime loader does not exist: {}",
            loader.to_string_lossy()
        )));
    }
    if !rootfsDir.is_dir() {
        return Err(HostError::new(format!(
            "Android managed runtime rootfs does not exist: {}",
            rootfsDir.to_string_lossy()
        )));
    }

    ensureRootfsAbsolutePath(&rootfsDir, &internalRoot)?;
    ensureRootfsAbsolutePath(&rootfsDir, &storageRoot)?;
    ensureRootfsAbsolutePath(&rootfsDir, Path::new("/data/local/tmp"))?;
    std::fs::create_dir_all(rootfsDir.join("dev/pts"))?;
    std::fs::create_dir_all(&tmpDir)?;

    let mut command = Command::new(&proot);
    command.current_dir(&runtimeDir);
    command.env("PROOT_TMP_DIR", tmpDir);
    command.env("PROOT_LOADER", loader);
    command.env("PROOT_NO_SECCOMP", "1");
    command.env("LD_LIBRARY_PATH", "");
    command.env("HOME", "/root");
    command.env("LANG", "C.UTF-8");
    command.env(
        "PATH",
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    );
    command.arg("-0");
    command.arg("-r").arg(&rootfsDir);
    command.arg("-b").arg("/proc");
    command.arg("-b").arg("/dev");
    command.arg("-b").arg("/sys");
    command.arg("-b").arg("/dev/pts");
    command.arg("-b").arg("/sdcard");
    command.arg("-b").arg("/storage");
    command.arg("-b").arg("/data/local/tmp:/data/local/tmp");
    command.arg("-b").arg(bindSamePath(&internalRoot));
    command.arg("-b").arg(bindSamePath(&storageRoot));
    command.arg("-w").arg("/root");
    command.arg(executable);
    Ok(command)
}

#[allow(non_snake_case)]
fn bindSamePath(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("{value}:{value}")
}

#[allow(non_snake_case)]
fn ensureRootfsAbsolutePath(rootfsDir: &Path, absolutePath: &Path) -> HostResult<()> {
    let value = absolutePath.to_string_lossy();
    if !value.starts_with('/') {
        return Err(HostError::new(format!(
            "Android managed runtime path must be absolute: {value}"
        )));
    }
    std::fs::create_dir_all(rootfsDir.join(value.trim_start_matches('/')))?;
    Ok(())
}
