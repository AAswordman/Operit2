use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;
use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_native_common::{NativePtyShellCommand, NativePtyTerminalHost};
use sha2::{Digest, Sha256};
use tar::Archive;
use uuid::Uuid;

const PLATFORM: &str = "ohos";
const NATIVE_TERMINAL: &str = "native";
const QEMU_VROOT_TERMINAL: &str = "qemu-vroot";
const SHELL_TERMINAL_TYPE: &str = "shell";
const QEMU_VROOT_EXECUTABLE: &str = "/data/app/bin/qemu-harmonix-aarch64";
const QEMU_VROOT_LIBRARY_DIRECTORY: &str = "/data/app/app.operit.mobile/harmonix_1.0/lib";
const VROOT_ASSET_DIRECTORY: &str = "ohos-vroot";
const VROOT_ROOTFS_DIRECTORY: &str = "alpine-3.23.6-aarch64";
const VROOT_ROOTFS_ARCHIVE: &str = "alpine-minirootfs-3.23.6-aarch64.tar.gz";
const VROOT_ROOTFS_SHA256: &str = "alpine-minirootfs-3.23.6-aarch64.tar.gz.sha256";
const VROOT_INSTALL_MARKER: &str = ".operit-rootfs.sha256";
const VROOT_HOST_WORKING_DIRECTORY_ENV: &str = "OPERIT_OHOS_HOST_WORKING_DIR";
pub(crate) const VROOT_HOST_MOUNT_ROOT: &str = "/mnt/host-root";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OhosTerminalBackend {
    NativeShell,
    QemuVroot,
}

#[derive(Clone)]
struct OhosVrootRuntime {
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    installationLock: Arc<Mutex<bool>>,
}

impl OhosVrootRuntime {
    /// Creates the OpenHarmony QEMU-vroot runtime paths for one app storage root.
    fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            runtimeRoot,
            workspaceRoot,
            installationLock: Arc::new(Mutex::new(false)),
        }
    }

    /// Returns the executable installed by the OpenHarmony HNP package.
    fn executablePath(&self) -> PathBuf {
        PathBuf::from(QEMU_VROOT_EXECUTABLE)
    }

    /// Returns the directory holding staged rootfs assets and installed rootfs data.
    fn assetDirectory(&self) -> PathBuf {
        self.runtimeRoot.join(VROOT_ASSET_DIRECTORY)
    }

    /// Returns the packaged Alpine archive copied into app runtime storage by ArkTS.
    fn rootfsArchivePath(&self) -> PathBuf {
        self.assetDirectory().join(VROOT_ROOTFS_ARCHIVE)
    }

    /// Returns the checksum file paired with the packaged Alpine archive.
    fn rootfsSha256Path(&self) -> PathBuf {
        self.assetDirectory().join(VROOT_ROOTFS_SHA256)
    }

    /// Returns the stable destination directory for the packaged Alpine root filesystem.
    fn rootfsDirectory(&self) -> PathBuf {
        self.assetDirectory().join(VROOT_ROOTFS_DIRECTORY)
    }

    /// Reports whether all immutable vroot runtime inputs are present.
    fn isAvailable(&self) -> bool {
        self.executablePath().is_file()
            && self.rootfsArchivePath().is_file()
            && self.rootfsSha256Path().is_file()
    }

    /// Extracts the checksum-verified Alpine root filesystem before a vroot session starts.
    fn ensureReady(&self) -> HostResult<()> {
        let mut installed = self
            .installationLock
            .lock()
            .map_err(|_| HostError::new("OpenHarmony vroot installation mutex poisoned"))?;
        if *installed {
            return Ok(());
        }
        let executable = self.executablePath();
        if !executable.is_file() {
            return Err(HostError::new(format!(
                "OpenHarmony qemu-vroot executable does not exist: {}",
                executable.display()
            )));
        }
        let archive = self.rootfsArchivePath();
        let expectedChecksum = readRootfsSha256(&self.rootfsSha256Path())?;
        let rootfsDirectory = self.rootfsDirectory();
        if rootfsDirectory.join(VROOT_INSTALL_MARKER).is_file()
            && fs::read_to_string(rootfsDirectory.join(VROOT_INSTALL_MARKER))?.trim()
                == expectedChecksum
        {
            verifyRootfsPayload(&rootfsDirectory)?;
            *installed = true;
            return Ok(());
        }
        verifyRootfsArchiveSha256(&archive, &expectedChecksum)?;
        let staging = self.assetDirectory().join(format!(
            ".{VROOT_ROOTFS_DIRECTORY}.installing-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&staging)?;
        let result = self.installRootfs(&archive, &staging, &rootfsDirectory, &expectedChecksum);
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        result?;
        *installed = true;
        Ok(())
    }

    /// Installs or upgrades bundled packages through apk without replacing the user's rootfs.
    fn installRootfs(
        &self,
        archive: &Path,
        staging: &Path,
        rootfs: &Path,
        checksum: &str,
    ) -> HostResult<()> {
        let payload = staging.join("payload");
        fs::create_dir(&payload)?;
        extractRootfsArchive(archive, &payload)?;
        let repository = staging.join("repository");
        fs::rename(payload.join("etc/operit/packages"), &repository)?;
        let repositoriesFile = staging.join("repositories");
        fs::rename(payload.join("etc/operit/repositories"), &repositoriesFile)?;
        let packages = fs::read_to_string(payload.join("etc/operit/packages.list"))?;
        let names: Vec<&str> = packages.split_whitespace().collect();
        if names.is_empty()
            || names.iter().any(|name| {
                !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            })
        {
            return Err(HostError::new("Invalid packaged Alpine package manifest"));
        }
        // QEMU creates physical absolute symlinks, so packages must install at the final path.
        if !rootfs.exists() {
            fs::rename(&payload, rootfs)?;
        }
        let destination = rootfs;
        let output = std::process::Command::new(self.executablePath())
            .current_dir(destination)
            .env("LD_LIBRARY_PATH", QEMU_VROOT_LIBRARY_DIRECTORY)
            .args([
                "-E",
                "LD_LIBRARY_PATH=",
                "-L",
                ".",
                "./sbin/apk",
                "--no-network",
                "--allow-untrusted",
                "--no-scripts",
                "--repositories-file",
            ])
            .arg(format!(
                "{VROOT_HOST_MOUNT_ROOT}{}",
                repositoriesFile.display()
            ))
            .arg("--repository")
            .arg(format!("{VROOT_HOST_MOUNT_ROOT}{}", repository.display()))
            .args(["add", "--upgrade"])
            .args(names)
            .output()?;
        if !output.status.success() {
            return Err(HostError::new(format!(
                "OpenHarmony Alpine package installation failed: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let status = std::process::Command::new(self.executablePath())
            .current_dir(destination)
            .env("LD_LIBRARY_PATH", QEMU_VROOT_LIBRARY_DIRECTORY)
            .args([
                "-E",
                "LD_LIBRARY_PATH=",
                "-L",
                ".",
                "./bin/busybox",
                "--install",
                "-s",
            ])
            .status()?;
        if !status.success() {
            return Err(HostError::new(
                "OpenHarmony Alpine shell command installation failed",
            ));
        }
        let resolver = self.assetDirectory().join("resolv.conf");
        if !resolver.exists() {
            fs::write(&resolver, "")?;
        }
        let guestResolver = destination.join("etc/resolv.conf");
        if guestResolver.symlink_metadata().is_ok() {
            fs::remove_file(&guestResolver)?;
        }
        std::os::unix::fs::symlink(&resolver, guestResolver)?;
        verifyRootfsPayload(destination)?;
        let marker = destination.join(format!("{VROOT_INSTALL_MARKER}.installing"));
        fs::write(&marker, checksum)?;
        fs::rename(marker, destination.join(VROOT_INSTALL_MARKER))?;
        Ok(())
    }
}

/// Hosts OpenHarmony native system-shell and QEMU-vroot Alpine terminal sessions.
#[derive(Clone)]
pub struct OhosTerminalHost {
    nativeShell: NativePtyTerminalHost,
    vrootShell: NativePtyTerminalHost,
    vrootRuntime: OhosVrootRuntime,
    sessionBackends: Arc<Mutex<BTreeMap<String, OhosTerminalBackend>>>,
}

impl OhosTerminalHost {
    /// Creates the OpenHarmony terminal host for explicit app runtime and workspace roots.
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> HostResult<Self> {
        let vrootRuntime = OhosVrootRuntime::new(runtimeRoot, workspaceRoot);
        let vrootShell = createVrootShell(&vrootRuntime)?;
        Ok(Self {
            nativeShell: NativePtyTerminalHost::systemShell(),
            vrootShell,
            vrootRuntime,
            sessionBackends: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    /// Resolves one public terminal implementation and terminal-type pair.
    fn backendForTerminal(
        &self,
        terminal: &str,
        terminalType: &str,
    ) -> HostResult<OhosTerminalBackend> {
        match (terminal.trim(), terminalType.trim()) {
            (NATIVE_TERMINAL, SHELL_TERMINAL_TYPE) => Ok(OhosTerminalBackend::NativeShell),
            (QEMU_VROOT_TERMINAL, SHELL_TERMINAL_TYPE) => Ok(OhosTerminalBackend::QemuVroot),
            (implementation, kind) => Err(HostError::new(format!(
                "Unsupported OpenHarmony terminal implementation and type: {implementation}/{kind}"
            ))),
        }
    }

    /// Resolves the recorded backend for one terminal session identifier.
    fn backendForSession(&self, sessionId: &str) -> HostResult<OhosTerminalBackend> {
        let sessions = self
            .sessionBackends
            .lock()
            .map_err(|_| HostError::new("OpenHarmony terminal session mutex poisoned"))?;
        sessions.get(sessionId).copied().ok_or_else(|| {
            HostError::new(format!(
                "OpenHarmony terminal session does not exist: {sessionId}"
            ))
        })
    }

    /// Records the backend that owns a live terminal session identifier.
    fn recordSession(&self, sessionId: &str, backend: OhosTerminalBackend) -> HostResult<()> {
        let mut sessions = self
            .sessionBackends
            .lock()
            .map_err(|_| HostError::new("OpenHarmony terminal session mutex poisoned"))?;
        match sessions.insert(sessionId.to_string(), backend) {
            Some(previous) if previous != backend => Err(HostError::new(format!(
                "OpenHarmony terminal session backend conflicts for {sessionId}"
            ))),
            _ => Ok(()),
        }
    }

    /// Removes a recorded terminal session after its owner closes it.
    fn removeSession(&self, sessionId: &str) -> HostResult<()> {
        let mut sessions = self
            .sessionBackends
            .lock()
            .map_err(|_| HostError::new("OpenHarmony terminal session mutex poisoned"))?;
        sessions.remove(sessionId).map(|_| ()).ok_or_else(|| {
            HostError::new(format!(
                "OpenHarmony terminal session does not exist: {sessionId}"
            ))
        })
    }

    /// Returns the shared PTY host owned by one OpenHarmony terminal backend.
    fn ptyHost(&self, backend: OhosTerminalBackend) -> &NativePtyTerminalHost {
        match backend {
            OhosTerminalBackend::NativeShell => &self.nativeShell,
            OhosTerminalBackend::QemuVroot => &self.vrootShell,
        }
    }

    /// Verifies that the requested OpenHarmony terminal backend is ready to start.
    fn prepareBackend(&self, backend: OhosTerminalBackend) -> HostResult<()> {
        match backend {
            OhosTerminalBackend::NativeShell => Ok(()),
            OhosTerminalBackend::QemuVroot => self.vrootRuntime.ensureReady(),
        }
    }
}

impl TerminalHost for OhosTerminalHost {
    /// Resolves QEMU Linux guest paths separately from native OpenHarmony paths.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        match self.backendForTerminal(terminal, terminalType)? {
            OhosTerminalBackend::QemuVroot => {
                operit_host_api::TerminalWorkingDirectory::resolveLinuxGuestDirectory(
                    workingDir,
                    resolveHostDirectory,
                )
            }
            OhosTerminalBackend::NativeShell => resolveHostDirectory(workingDir),
        }
    }

    /// Returns OpenHarmony native-shell and QEMU-vroot terminal capabilities.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: QEMU_VROOT_TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            types: vec![
                TerminalTypeInfo {
                    terminal: QEMU_VROOT_TERMINAL.to_string(),
                    terminalType: SHELL_TERMINAL_TYPE.to_string(),
                    available: self.vrootRuntime.isAvailable(),
                    description: "OpenHarmony QEMU-vroot Alpine Linux shell".to_string(),
                },
                TerminalTypeInfo {
                    terminal: NATIVE_TERMINAL.to_string(),
                    terminalType: SHELL_TERMINAL_TYPE.to_string(),
                    available: Path::new("/bin/sh").is_file(),
                    description: "OpenHarmony system /bin/sh terminal".to_string(),
                },
            ],
        })
    }

    /// Starts a PTY session in one exact OpenHarmony terminal implementation.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let backend = self.backendForTerminal(terminal, terminalType)?;
        self.prepareBackend(backend)?;
        if backend == OhosTerminalBackend::QemuVroot && !Path::new(workingDir).is_absolute() {
            return Err(HostError::new(format!(
                "OpenHarmony qemu-vroot working directory must be absolute: {workingDir}"
            )));
        }
        let sessionId = self.ptyHost(backend).startPtySession(
            sessionName,
            NATIVE_TERMINAL,
            SHELL_TERMINAL_TYPE,
            workingDir,
            rows,
            cols,
        )?;
        self.recordSession(&sessionId, backend)?;
        Ok(sessionId)
    }

    /// Reads pending PTY bytes from a recorded OpenHarmony terminal session.
    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend).readPtySession(sessionId)
    }

    /// Writes PTY bytes to a recorded OpenHarmony terminal session.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend).writePtySession(sessionId, data)
    }

    /// Resizes one recorded OpenHarmony terminal PTY.
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend)
            .resizePtySession(sessionId, rows, cols)
    }

    /// Polls one recorded OpenHarmony terminal PTY for an exit code.
    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend).pollPtyExitCode(sessionId)
    }

    /// Closes one recorded OpenHarmony terminal PTY.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend).closePtySession(sessionId)?;
        self.removeSession(sessionId)
    }

    /// Lists all visible OpenHarmony native-shell and QEMU-vroot sessions.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let nativeSessions = self.nativeShell.listSessions()?;
        let vrootSessions = self.vrootShell.listSessions()?;
        let mut sessions = Vec::with_capacity(nativeSessions.len() + vrootSessions.len());
        for session in nativeSessions {
            self.recordSession(&session.sessionId, OhosTerminalBackend::NativeShell)?;
            sessions.push(mapSessionEntry(session, OhosTerminalBackend::NativeShell));
        }
        for session in vrootSessions {
            self.recordSession(&session.sessionId, OhosTerminalBackend::QemuVroot)?;
            sessions.push(mapSessionEntry(session, OhosTerminalBackend::QemuVroot));
        }
        Ok(sessions)
    }

    /// Creates or returns a named QEMU-vroot Alpine session used by tools and plugins.
    fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTypedSession(sessionName, SHELL_TERMINAL_TYPE)
    }

    /// Creates or reuses the requested interpreter in the configured QEMU-vroot backend.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let backend = self.backendForTerminal(QEMU_VROOT_TERMINAL, terminalType)?;
        self.prepareBackend(backend)?;
        let session = self
            .vrootShell
            .createOrGetTypedSession(sessionName, terminalType)?;
        self.recordSession(&session.sessionId, backend)?;
        Ok(mapSessionInfo(session, backend))
    }

    /// Executes one command in a recorded OpenHarmony terminal session.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend)
            .executeInSession(sessionId, command, timeoutMs)
            .map(|output| mapCommandOutput(output, backend))
    }

    /// Executes one hidden command in the primary QEMU-vroot Alpine environment.
    fn executeInSessionStreaming(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend)
            .executeInSessionStreaming(sessionId, command, timeoutMs, onOutput)
            .map(|output| mapCommandOutput(output, backend))
    }

    /// Executes one hidden command in the primary QEMU-vroot Alpine environment.
    fn executeHiddenCommand(
        &self,
        command: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let backend = OhosTerminalBackend::QemuVroot;
        self.prepareBackend(backend)?;
        self.vrootShell
            .executeHiddenCommand(command, executorKey, timeoutMs)
            .map(|output| mapHiddenCommandOutput(output, backend))
    }

    /// Sends text or a control sequence to a recorded OpenHarmony terminal session.
    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend)
            .inputInSession(sessionId, input, control)
    }

    /// Closes one recorded OpenHarmony terminal session.
    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let backend = self.backendForSession(sessionId)?;
        let output = self.ptyHost(backend).closeSession(sessionId)?;
        self.removeSession(sessionId)?;
        Ok(output)
    }

    /// Reads the screen state from a recorded OpenHarmony terminal session.
    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        let backend = self.backendForSession(sessionId)?;
        self.ptyHost(backend)
            .getSessionScreen(sessionId)
            .map(|output| mapScreenOutput(output, backend))
    }
}

/// Creates the QEMU-vroot Alpine PTY launcher with a fixed rootfs process directory.
fn createVrootShell(runtime: &OhosVrootRuntime) -> HostResult<NativePtyTerminalHost> {
    let rootfsDirectory = runtime.rootfsDirectory();
    NativePtyTerminalHost::customShell(NativePtyShellCommand {
        program: runtime.executablePath().to_string_lossy().to_string(),
        arguments: vec![
            "-E".to_string(),
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
            "-E".to_string(),
            "HOME=/root".to_string(),
            "-E".to_string(),
            "LD_LIBRARY_PATH=".to_string(),
            "-L".to_string(),
            ".".to_string(),
            "./bin/busybox".to_string(),
            "sh".to_string(),
            "-c".to_string(),
            format!(
                "cd \"{VROOT_HOST_MOUNT_ROOT}$OPERIT_OHOS_HOST_WORKING_DIR\" && exec /bin/sh -i"
            ),
        ],
        description: "OpenHarmony QEMU-vroot Alpine Linux shell".to_string(),
        processWorkingDirectory: rootfsDirectory.to_string_lossy().to_string(),
        defaultSessionWorkingDirectory: runtime.workspaceRoot.to_string_lossy().to_string(),
        environment: vec![(
            "LD_LIBRARY_PATH".to_string(),
            QEMU_VROOT_LIBRARY_DIRECTORY.to_string(),
        )],
        sessionWorkingDirectoryEnvironment: VROOT_HOST_WORKING_DIRECTORY_ENV.to_string(),
    })
}

/// Returns the public implementation name for one OpenHarmony terminal backend.
fn terminalName(backend: OhosTerminalBackend) -> &'static str {
    match backend {
        OhosTerminalBackend::NativeShell => NATIVE_TERMINAL,
        OhosTerminalBackend::QemuVroot => QEMU_VROOT_TERMINAL,
    }
}

/// Maps an inner terminal session descriptor to its OpenHarmony public identity.
fn mapSessionInfo(
    mut session: TerminalSessionInfo,
    backend: OhosTerminalBackend,
) -> TerminalSessionInfo {
    session.platform = PLATFORM.to_string();
    session.terminal = terminalName(backend).to_string();
    session.terminalType = SHELL_TERMINAL_TYPE.to_string();
    session
}

/// Maps an inner terminal command result to its OpenHarmony public identity.
fn mapCommandOutput(
    mut output: TerminalCommandOutput,
    backend: OhosTerminalBackend,
) -> TerminalCommandOutput {
    output.platform = PLATFORM.to_string();
    output.terminal = terminalName(backend).to_string();
    output.terminalType = SHELL_TERMINAL_TYPE.to_string();
    output
}

/// Maps an inner hidden terminal command result to its OpenHarmony public identity.
fn mapHiddenCommandOutput(
    mut output: HiddenTerminalCommandOutput,
    backend: OhosTerminalBackend,
) -> HiddenTerminalCommandOutput {
    output.platform = PLATFORM.to_string();
    output.terminal = terminalName(backend).to_string();
    output.terminalType = SHELL_TERMINAL_TYPE.to_string();
    output
}

/// Maps an inner terminal screen result to its OpenHarmony public identity.
fn mapScreenOutput(
    mut output: TerminalScreenOutput,
    backend: OhosTerminalBackend,
) -> TerminalScreenOutput {
    output.platform = PLATFORM.to_string();
    output.terminal = terminalName(backend).to_string();
    output.terminalType = SHELL_TERMINAL_TYPE.to_string();
    output
}

/// Maps an inner terminal session listing entry to its OpenHarmony public identity.
fn mapSessionEntry(
    mut session: TerminalSessionListEntry,
    backend: OhosTerminalBackend,
) -> TerminalSessionListEntry {
    session.platform = PLATFORM.to_string();
    session.terminal = terminalName(backend).to_string();
    session.terminalType = SHELL_TERMINAL_TYPE.to_string();
    session
}

/// Reads the canonical SHA-256 value from one packaged checksum file.
fn readRootfsSha256(path: &Path) -> HostResult<String> {
    let content = fs::read_to_string(path).map_err(|error| {
        HostError::new(format!(
            "OpenHarmony vroot rootfs checksum cannot be read: {}: {error}",
            path.to_string_lossy()
        ))
    })?;
    let value = content.split_whitespace().next().ok_or_else(|| {
        HostError::new(format!(
            "OpenHarmony vroot rootfs checksum is empty: {}",
            path.to_string_lossy()
        ))
    })?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostError::new(format!(
            "OpenHarmony vroot rootfs checksum is invalid: {}",
            path.to_string_lossy()
        )));
    }
    Ok(value.to_ascii_lowercase())
}

/// Verifies that one packaged rootfs archive has the expected SHA-256 value.
fn verifyRootfsArchiveSha256(path: &Path, expectedChecksum: &str) -> HostResult<()> {
    let mut reader = BufReader::new(File::open(path).map_err(|error| {
        HostError::new(format!(
            "OpenHarmony vroot rootfs archive cannot be opened: {}: {error}",
            path.to_string_lossy()
        ))
    })?);
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let actualChecksum = format!("{:x}", digest.finalize());
    if actualChecksum != expectedChecksum {
        return Err(HostError::new(format!(
            "OpenHarmony vroot rootfs checksum mismatch: expected {expectedChecksum}, got {actualChecksum}"
        )));
    }
    Ok(())
}

/// Extracts one gzip-compressed Alpine rootfs archive into a newly created directory.
fn extractRootfsArchive(archivePath: &Path, destination: &Path) -> HostResult<()> {
    let archiveFile = File::open(archivePath)?;
    let decoder = GzDecoder::new(BufReader::new(archiveFile));
    let mut archive = Archive::new(decoder);
    archive.unpack(destination).map_err(|error| {
        HostError::new(format!(
            "OpenHarmony vroot rootfs extraction failed: {}",
            error
        ))
    })
}

/// Verifies the runtimes supplied by the offline Alpine image.
fn verifyRootfsPayload(rootfsDirectory: &Path) -> HostResult<()> {
    for relative in [
        "bin/busybox",
        "sbin/apk",
        "usr/bin/python3",
        "usr/bin/node",
        "usr/bin/uv",
        "usr/bin/pnpm",
        "usr/bin/tesseract",
        "usr/bin/pdftoppm",
    ] {
        if !rootfsDirectory.join(relative).is_file() {
            return Err(HostError::new(format!(
                "OpenHarmony rootfs executable is missing: {relative}"
            )));
        }
    }
    Ok(())
}
