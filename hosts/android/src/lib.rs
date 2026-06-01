#![allow(non_snake_case)]

use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::ffi::CString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, FileEntry,
    FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest, GrepCodeResult,
    HiddenTerminalCommandOutput, HostEnvironmentDescriptor, HostError, HostResult, HttpHost,
    HttpRequestData, HttpResponseData, LocationData, ManagedRuntimeHost, ManagedRuntimeProcess,
    ManagedRuntimeProgram, NotificationData, RuntimeCommandOutput, RuntimeProcessRequest,
    RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry, RuntimeStorageHost,
    SystemOperationHost, SystemSettingData, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalTypeInfo, WebVisitHost, WebVisitRequest, WebVisitResult,
};

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);
type RawFd = i32;
#[cfg(target_os = "android")]
type AndroidPid = libc::pid_t;
#[cfg(not(target_os = "android"))]
type AndroidPid = i32;

#[cfg(target_os = "android")]
#[link(name = "log")]
extern "C" {
    fn __android_log_write(
        priority: libc::c_int,
        tag: *const libc::c_char,
        text: *const libc::c_char,
    ) -> libc::c_int;
}

#[derive(Clone, Debug, Default)]
pub struct AndroidFileSystemHost {
    inner: operit_host_linux_native::LinuxFileSystemHost,
}

impl AndroidFileSystemHost {
    pub fn new() -> Self {
        Self {
            inner: operit_host_linux_native::LinuxFileSystemHost::new(),
        }
    }
}

impl FileSystemHost for AndroidFileSystemHost {
    fn envLabel(&self) -> &str {
        "android"
    }

    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::android()
    }

    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        if path.trim().is_empty() {
            return Err(HostError::new(format!("{paramName} parameter is required")));
        }
        if !std::path::Path::new(path).is_absolute() {
            return Err(HostError::new(format!(
                "Invalid path: '{path}'. Path must be an absolute Android path."
            )));
        }
        Ok(())
    }

    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        self.inner.listFiles(path)
    }

    fn readFile(&self, path: &str) -> HostResult<String> {
        self.inner.readFile(path)
    }

    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        self.inner.readFileWithLimit(path, maxBytes)
    }

    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readFileBytes(path)
    }

    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        self.inner.writeFile(path, content, append)
    }

    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeFileBytes(path, content)
    }

    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.deleteFile(path, recursive)
    }

    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        self.inner.fileExists(path)
    }

    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.moveFile(source, destination)
    }

    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        self.inner.copyFile(source, destination, recursive)
    }

    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        self.inner.makeDirectory(path, createParents)
    }

    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> {
        self.inner.findFiles(request)
    }

    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        self.inner.fileInfo(path)
    }

    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        self.inner.grepCode(request)
    }

    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.zipFiles(source, destination)
    }

    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.unzipFiles(source, destination)
    }

    fn openFile(&self, path: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android open_file requires the Flutter Android host bridge: {path}"
        )))
    }

    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android share_file requires the Flutter Android host bridge: {path} ({title})"
        )))
    }
}

#[derive(Clone, Debug)]
pub struct AndroidRuntimeStorageHost {
    root: PathBuf,
    inner: operit_host_linux_native::LinuxRuntimeStorageHost,
}

impl AndroidRuntimeStorageHost {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: root.clone(),
            inner: operit_host_linux_native::LinuxRuntimeStorageHost::new(root),
        }
    }
}

impl RuntimeStorageHost for AndroidRuntimeStorageHost {
    fn rootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(path)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }

    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }

    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

impl RuntimeSqliteHost for AndroidRuntimeStorageHost {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(path)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AndroidHttpHost {
    inner: operit_host_linux_native::LinuxHttpHost,
}

impl AndroidHttpHost {
    pub fn new() -> Self {
        Self {
            inner: operit_host_linux_native::LinuxHttpHost::new(),
        }
    }
}

impl HttpHost for AndroidHttpHost {
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        let inner = self.inner.clone();
        thread::spawn(move || inner.executeHttpRequest(request))
            .join()
            .map_err(|_| HostError::new("android HTTP request thread panicked"))?
    }
}

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

#[allow(non_snake_case)]
fn requiredAndroidRuntimePath(name: &str) -> HostResult<PathBuf> {
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
fn validateRootfsExecutable(executable: &str) -> HostResult<()> {
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
fn buildAndroidProotCommand(executable: &str, cwd: Option<&str>) -> HostResult<Command> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let rootfsDir = requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?;
    let storageRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_STORAGE_ROOT")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let proot = requiredAndroidRuntimePath("OPERIT_ANDROID_PROOT")?;
    let loader = requiredAndroidRuntimePath("OPERIT_ANDROID_LOADER")?;
    let nativeLibraryDir = requiredAndroidRuntimePath("OPERIT_ANDROID_NATIVE_LIBRARY_DIR")?;

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
    std::fs::create_dir_all(&tmpDir)?;

    let workDir = match cwd.map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => "/home/operit".to_string(),
    };

    let mut command = Command::new(&proot);
    command.current_dir(&runtimeDir);
    command.env("PROOT_TMP_DIR", tmpDir);
    command.env("PROOT_LOADER", loader);
    command.env("PROOT_NO_SECCOMP", "1");
    command.env(
        "LD_LIBRARY_PATH",
        format!(
            "{}:{}",
            nativeLibraryDir.to_string_lossy(),
            runtimeDir.to_string_lossy()
        ),
    );
    command.env("HOME", "/home/operit");
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
    command.arg("-b").arg("/sdcard");
    command.arg("-b").arg("/storage");
    command.arg("-b").arg(bindSamePath(&internalRoot));
    command.arg("-w").arg(workDir);
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

#[derive(Clone, Default)]
pub struct AndroidTerminalHost {
    state: Arc<Mutex<AndroidTerminalState>>,
}

#[derive(Default)]
struct AndroidTerminalState {
    sessions: BTreeMap<String, AndroidTerminalSession>,
    sessionNameToId: BTreeMap<String, String>,
    hiddenExecutorKeyToSessionId: BTreeMap<String, String>,
    ptySessions: BTreeMap<String, AndroidPtySession>,
}

struct AndroidTerminalSession {
    id: String,
    name: String,
    terminalType: String,
    child: Child,
    stdin: ChildStdin,
    stdoutRx: Receiver<String>,
    stderrLines: Arc<Mutex<VecDeque<String>>>,
    screenLines: VecDeque<String>,
    commandRunning: bool,
}

struct AndroidPtySession {
    pid: AndroidPid,
    masterFd: RawFd,
    exitCode: Option<i32>,
}

impl Drop for AndroidPtySession {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        unsafe {
            libc::kill(self.pid, libc::SIGHUP);
            libc::kill(self.pid, libc::SIGKILL);
            libc::close(self.masterFd);
        }
    }
}

impl AndroidTerminalHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn startPtySession(&self, workingDir: &str, rows: u16, cols: u16) -> HostResult<String> {
        let command = buildAndroidPtyCommand(workingDir)?;
        let (pid, masterFd) = forkPtyExecve(&command, rows, cols)?;
        let sessionId = nextTerminalId();
        let mut state = self.lockState()?;
        state.ptySessions.insert(
            sessionId.clone(),
            AndroidPtySession {
                pid,
                masterFd,
                exitCode: None,
            },
        );
        Ok(sessionId)
    }

    pub fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        let mut state = self.lockState()?;
        let session = state.ptySessions.get_mut(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        let data = readPtyFd(session.masterFd)?;
        Ok(data)
    }

    pub fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        let state = self.lockState()?;
        let session = state.ptySessions.get(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        writePtyFd(session.masterFd, data)
    }

    pub fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        let state = self.lockState()?;
        let session = state.ptySessions.get(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        setPtyWindowSize(session.masterFd, rows, cols)
    }

    pub fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        let mut state = self.lockState()?;
        let session = state.ptySessions.get_mut(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        if let Some(exitCode) = session.exitCode {
            return Ok(Some(exitCode));
        }
        let exitCode = pollPidExitCode(session.pid)?;
        if let Some(code) = exitCode {
            session.exitCode = Some(code);
        }
        Ok(exitCode)
    }

    pub fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        let mut state = self.lockState()?;
        let removed = state.ptySessions.remove(sessionId);
        if removed.is_none() {
            androidLogError(&format!("closePtySession missing sessionId={sessionId}"));
            return Err(HostError::new(format!(
                "Android PTY session does not exist: {sessionId}"
            )));
        }
        Ok(())
    }

    pub fn terminalDebugInfo(&self, workingDir: &str) -> HostResult<BTreeMap<String, String>> {
        androidTerminalDebugInfo(workingDir)
    }

    fn lockState(&self) -> HostResult<std::sync::MutexGuard<'_, AndroidTerminalState>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("android terminal state mutex poisoned"))
    }
}

impl TerminalHost for AndroidTerminalHost {
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "android".to_string(),
            defaultType: "android".to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: "android".to_string(),
                available: true,
                description: "Android proot terminal".to_string(),
            }],
        })
    }

    fn createOrGetSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let normalizedTerminalType = normalizeAndroidTerminalType(terminalType)?;
        let key = sessionKey(&normalizedTerminalType, &normalizedSessionName);
        let mut state = self.lockState()?;
        if let Some(sessionId) = state.sessionNameToId.get(&key).cloned() {
            if state.sessions.contains_key(&sessionId) {
                return Ok(TerminalSessionInfo {
                    sessionId,
                    sessionName: normalizedSessionName,
                    terminalType: normalizedTerminalType,
                    isNewSession: false,
                });
            }
            state.sessionNameToId.remove(&key);
        }

        let session = createAndroidShellSession(
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
        )?;
        let sessionId = session.id.clone();
        state.sessionNameToId.insert(key, sessionId.clone());
        state.sessions.insert(sessionId.clone(), session);
        Ok(TerminalSessionInfo {
            sessionId,
            sessionName: normalizedSessionName,
            terminalType: normalizedTerminalType,
            isNewSession: true,
        })
    }

    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let normalizedCommand = nonBlank(command, "command")?;
        let mut state = self.lockState()?;
        let session = state
            .sessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let result = executeAndroidShellCommandInSession(session, &normalizedCommand, timeoutMs)?;
        Ok(TerminalCommandOutput {
            command: normalizedCommand,
            output: result.output,
            exitCode: result.exitCode,
            sessionId: normalizedSessionId,
            terminalType: session.terminalType.clone(),
            timedOut: result.timedOut,
        })
    }

    fn executeHiddenCommand(
        &self,
        command: &str,
        terminalType: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let normalizedCommand = nonBlank(command, "command")?;
        let normalizedTerminalType = normalizeAndroidTerminalType(terminalType)?;
        let normalizedExecutorKey = match executorKey.trim() {
            "" => "default".to_string(),
            value => value.to_string(),
        };
        let key = sessionKey(&normalizedTerminalType, &normalizedExecutorKey);
        let mut state = self.lockState()?;
        let sessionId = match state.hiddenExecutorKeyToSessionId.get(&key).cloned() {
            Some(sessionId) if state.sessions.contains_key(&sessionId) => sessionId,
            Some(sessionId) => {
                state.hiddenExecutorKeyToSessionId.remove(&key);
                let _ = sessionId;
                let session = createAndroidShellSession(
                    format!("hidden:{normalizedExecutorKey}"),
                    normalizedTerminalType.clone(),
                )?;
                let sessionId = session.id.clone();
                state
                    .hiddenExecutorKeyToSessionId
                    .insert(key.clone(), sessionId.clone());
                state.sessions.insert(sessionId.clone(), session);
                sessionId
            }
            None => {
                let session = createAndroidShellSession(
                    format!("hidden:{normalizedExecutorKey}"),
                    normalizedTerminalType.clone(),
                )?;
                let sessionId = session.id.clone();
                state
                    .hiddenExecutorKeyToSessionId
                    .insert(key, sessionId.clone());
                state.sessions.insert(sessionId.clone(), session);
                sessionId
            }
        };
        let session = state.sessions.get_mut(&sessionId).ok_or_else(|| {
            HostError::new(format!("Hidden terminal session missing: {sessionId}"))
        })?;
        let result = executeAndroidShellCommandInSession(session, &normalizedCommand, timeoutMs)?;
        Ok(HiddenTerminalCommandOutput {
            command: normalizedCommand,
            output: result.output,
            exitCode: result.exitCode,
            executorKey: normalizedExecutorKey,
            terminalType: normalizedTerminalType,
            timedOut: result.timedOut,
        })
    }

    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        if input.is_none() && control.and_then(normalizeControl).is_none() {
            return Err(HostError::new(
                "At least one of input or control is required",
            ));
        }
        let mut state = self.lockState()?;
        let session = state
            .sessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let acceptedChars = applyTerminalInput(session, input, control.and_then(normalizeControl))?;
        Ok(TerminalInputOutput {
            sessionId: normalizedSessionId,
            acceptedChars,
        })
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let mut state = self.lockState()?;
        let mut session = state.sessions.remove(&normalizedSessionId).ok_or_else(|| {
            HostError::new(format!("Terminal session does not exist: {sessionId}"))
        })?;
        let _ = session.child.kill();
        state
            .sessionNameToId
            .retain(|_, value| value != &normalizedSessionId);
        state
            .hiddenExecutorKeyToSessionId
            .retain(|_, value| value != &normalizedSessionId);
        Ok(TerminalCloseOutput {
            sessionId: normalizedSessionId.clone(),
            success: true,
            message: format!("Terminal session closed: {normalizedSessionId}"),
        })
    }

    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let state = self.lockState()?;
        let session = state.sessions.get(&normalizedSessionId).ok_or_else(|| {
            HostError::new(format!("Terminal session does not exist: {sessionId}"))
        })?;
        let content = session
            .screenLines
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let rows = session.screenLines.len();
        let cols = match session
            .screenLines
            .iter()
            .map(|line| line.chars().count())
            .max()
        {
            Some(value) => value,
            None => 0,
        };
        Ok(TerminalScreenOutput {
            sessionId: normalizedSessionId,
            terminalType: session.terminalType.clone(),
            rows,
            cols,
            content,
            commandRunning: session.commandRunning,
        })
    }
}

struct AndroidSessionCommandResult {
    output: String,
    exitCode: i32,
    timedOut: bool,
}

fn createAndroidShellSession(
    name: String,
    terminalType: String,
) -> HostResult<AndroidTerminalSession> {
    let mut command = buildAndroidProotCommand("/bin/bash", Some("/home/operit"))?;
    command.arg("-l");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| HostError::new("terminal shell has no stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| HostError::new("terminal shell has no stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| HostError::new("terminal shell has no stderr"))?;
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
    Ok(AndroidTerminalSession {
        id: nextTerminalId(),
        name,
        terminalType,
        child,
        stdin,
        stdoutRx,
        stderrLines,
        screenLines: VecDeque::new(),
        commandRunning: false,
    })
}

fn executeAndroidShellCommandInSession(
    session: &mut AndroidTerminalSession,
    command: &str,
    timeoutMs: u64,
) -> HostResult<AndroidSessionCommandResult> {
    let marker = format!(
        "__OPERIT_TERMINAL_{}__",
        NEXT_TERMINAL_ID.fetch_add(1, Ordering::SeqCst)
    );
    let endMarkerPrefix = format!("{marker}_END:");
    let script = format!(
        "printf '%s\\n' '{marker}_START'\n{{\n{command}\n}}\n__operit_exit_code=$?\nprintf '%s%s\\n' '{endMarkerPrefix}' \"$__operit_exit_code\"\n"
    );
    session.commandRunning = true;
    session.stdin.write_all(script.as_bytes())?;
    session.stdin.flush()?;

    let deadline = Duration::from_millis(timeoutMs);
    let start = SystemTime::now();
    let mut outputLines = Vec::new();
    let mut sawStart = false;
    loop {
        let elapsed = match start.elapsed() {
            Ok(value) => value,
            Err(_) => deadline,
        };
        if elapsed >= deadline {
            session.commandRunning = false;
            let output = joinOutput(outputLines, drainAndroidStderr(session)?);
            appendAndroidScreenLines(session, &output);
            return Ok(AndroidSessionCommandResult {
                output,
                exitCode: -1,
                timedOut: true,
            });
        }
        let remaining = deadline - elapsed;
        let wait = remaining.min(Duration::from_millis(100));
        match session.stdoutRx.recv_timeout(wait) {
            Ok(line) => {
                if line == format!("{marker}_START") {
                    sawStart = true;
                    continue;
                }
                if sawStart && line.starts_with(&endMarkerPrefix) {
                    session.commandRunning = false;
                    let exitCodeText = line[endMarkerPrefix.len()..].trim();
                    let exitCode = match exitCodeText.parse::<i32>() {
                        Ok(value) => value,
                        Err(_) => -1,
                    };
                    let output = joinOutput(outputLines, drainAndroidStderr(session)?);
                    appendAndroidScreenLines(session, &output);
                    return Ok(AndroidSessionCommandResult {
                        output,
                        exitCode,
                        timedOut: false,
                    });
                }
                if sawStart {
                    outputLines.push(line);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                session.commandRunning = false;
                return Err(HostError::new(format!(
                    "Terminal session '{}' closed while executing command",
                    session.name
                )));
            }
        }
    }
}

fn drainAndroidStderr(session: &AndroidTerminalSession) -> HostResult<Vec<String>> {
    let mut lines = session
        .stderrLines
        .lock()
        .map_err(|_| HostError::new("terminal stderr mutex poisoned"))?;
    let mut collected = Vec::new();
    while let Some(line) = lines.pop_front() {
        collected.push(line);
    }
    Ok(collected)
}

fn joinOutput(mut stdoutLines: Vec<String>, stderrLines: Vec<String>) -> String {
    stdoutLines.extend(stderrLines);
    stdoutLines.join("\n")
}

fn appendAndroidScreenLines(session: &mut AndroidTerminalSession, output: &str) {
    for line in output.lines() {
        session.screenLines.push_back(line.to_string());
        while session.screenLines.len() > 200 {
            session.screenLines.pop_front();
        }
    }
}

fn applyTerminalInput(
    session: &mut AndroidTerminalSession,
    input: Option<&str>,
    control: Option<&str>,
) -> HostResult<usize> {
    let mut acceptedChars = 0;
    if let Some(input) = input {
        session.stdin.write_all(input.as_bytes())?;
        acceptedChars += input.chars().count();
    }
    if let Some(control) = control {
        let sequence = controlToSequence(control, input)?;
        session.stdin.write_all(sequence.as_bytes())?;
        acceptedChars += sequence.chars().count();
    }
    session.stdin.flush()?;
    Ok(acceptedChars)
}

fn controlToSequence(control: &str, input: Option<&str>) -> HostResult<String> {
    match control {
        "enter" => Ok("\n".to_string()),
        "tab" => Ok("\t".to_string()),
        "esc" => Ok("\x1b".to_string()),
        "up" => Ok("\x1b[A".to_string()),
        "down" => Ok("\x1b[B".to_string()),
        "right" => Ok("\x1b[C".to_string()),
        "left" => Ok("\x1b[D".to_string()),
        "home" => Ok("\x1b[H".to_string()),
        "end" => Ok("\x1b[F".to_string()),
        "pageup" => Ok("\x1b[5~".to_string()),
        "pagedown" => Ok("\x1b[6~".to_string()),
        "delete" => Ok("\x1b[3~".to_string()),
        "backspace" => Ok("\x7f".to_string()),
        "ctrl" | "control" => ctrlSequence(input),
        "alt" | "meta" | "cmd" => Ok(format!("\x1b{}", input.unwrap_or(""))),
        "shift" => Ok(input.unwrap_or("").to_uppercase()),
        other => Err(HostError::new(format!(
            "Unsupported terminal control: {other}"
        ))),
    }
}

fn ctrlSequence(input: Option<&str>) -> HostResult<String> {
    let input = input.ok_or_else(|| HostError::new("ctrl control requires input"))?;
    let mut chars = input.chars();
    let value = chars
        .next()
        .ok_or_else(|| HostError::new("ctrl control requires input"))?;
    if chars.next().is_some() {
        return Err(HostError::new(
            "ctrl control input must be a single character",
        ));
    }
    let code = match value.to_ascii_uppercase() {
        'A'..='Z' => value.to_ascii_uppercase() as u8 - b'A' + 1,
        '[' => 27,
        '\\' => 28,
        ']' => 29,
        '^' => 30,
        '_' => 31,
        '?' => 127,
        other => {
            return Err(HostError::new(format!(
                "Unsupported ctrl control input: {other}"
            )))
        }
    };
    Ok((code as char).to_string())
}

fn normalizeControl(rawControl: &str) -> Option<&'static str> {
    match rawControl.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "return" => Some("enter"),
        "escape" => Some("esc"),
        "arrowup" => Some("up"),
        "arrowdown" => Some("down"),
        "arrowleft" => Some("left"),
        "arrowright" => Some("right"),
        "pgup" | "page_up" => Some("pageup"),
        "pgdn" | "page_down" => Some("pagedown"),
        "del" => Some("delete"),
        "enter" => Some("enter"),
        "tab" => Some("tab"),
        "esc" => Some("esc"),
        "up" => Some("up"),
        "down" => Some("down"),
        "left" => Some("left"),
        "right" => Some("right"),
        "home" => Some("home"),
        "end" => Some("end"),
        "pageup" => Some("pageup"),
        "pagedown" => Some("pagedown"),
        "delete" => Some("delete"),
        "backspace" => Some("backspace"),
        "ctrl" | "control" => Some("ctrl"),
        "alt" => Some("alt"),
        "shift" => Some("shift"),
        "meta" => Some("meta"),
        "cmd" => Some("cmd"),
        _ => None,
    }
}

fn normalizeAndroidTerminalType(terminalType: &str) -> HostResult<String> {
    match terminalType.trim() {
        "" | "android" => Ok("android".to_string()),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for android host: {value}"
        ))),
    }
}

fn nonBlank(value: &str, paramName: &str) -> HostResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HostError::new(format!("{paramName} parameter is required")));
    }
    Ok(trimmed.to_string())
}

#[cfg(target_os = "android")]
fn androidLogError(message: &str) {
    androidLogWrite(6, message);
}

#[cfg(not(target_os = "android"))]
fn androidLogError(_message: &str) {}

#[cfg(target_os = "android")]
fn androidLogWrite(priority: libc::c_int, message: &str) {
    if let (Ok(tag), Ok(text)) = (CString::new("OperitTerminal"), CString::new(message)) {
        unsafe {
            let _ = __android_log_write(priority, tag.as_ptr(), text.as_ptr());
        }
    }
}

fn sessionKey(terminalType: &str, name: &str) -> String {
    format!("{terminalType}:{name}")
}

fn nextTerminalId() -> String {
    let millis = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_millis(),
        Err(_) => 0,
    };
    let sequence = NEXT_TERMINAL_ID.fetch_add(1, Ordering::SeqCst);
    format!("terminal-{millis}-{sequence}")
}

struct AndroidPtyCommand {
    executable: CString,
    argv: Vec<CString>,
    envp: Vec<CString>,
    cwd: CString,
}

fn buildAndroidPtyCommand(workingDir: &str) -> HostResult<AndroidPtyCommand> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let bash = requiredAndroidRuntimePath("OPERIT_ANDROID_BASH")?;
    let loader = requiredAndroidRuntimePath("OPERIT_ANDROID_LOADER")?;
    let nativeLibraryDir = requiredAndroidRuntimePath("OPERIT_ANDROID_NATIVE_LIBRARY_DIR")?;

    std::fs::create_dir_all(&tmpDir)?;

    let workDir = nonBlank(workingDir, "working_directory")?;
    let ldLibraryPath = format!(
        "{}:{}",
        nativeLibraryDir.to_string_lossy(),
        runtimeDir.to_string_lossy()
    );
    let systemPath = env::var("PATH")
        .map_err(|error| HostError::new(format!("Android terminal PATH is required: {error}")))?;
    let argv = vec![
        cstringPath(&bash)?,
        cstring("-c")?,
        cstring("source $HOME/common.sh && start_shell")?,
    ];
    let envp = vec![
        cstring(&format!(
            "PATH={}:{}",
            runtimeDir.to_string_lossy(),
            systemPath
        ))?,
        cstring(&format!("HOME={}", internalRoot.to_string_lossy()))?,
        cstring(&format!("PREFIX={}", runtimeDir.to_string_lossy()))?,
        cstring(&format!("TERMUX_PREFIX={}", runtimeDir.to_string_lossy()))?,
        cstring(&format!("LD_LIBRARY_PATH={ldLibraryPath}"))?,
        cstring(&format!("PROOT_LOADER={}", loader.to_string_lossy()))?,
        cstring("PROOT_NO_SECCOMP=1")?,
        cstring(&format!("TMPDIR={}", tmpDir.to_string_lossy()))?,
        cstring(&format!("PROOT_TMP_DIR={}", tmpDir.to_string_lossy()))?,
        cstring(&format!("OPERIT_WORKING_DIR={workDir}"))?,
        cstring("TERM=xterm-256color")?,
        cstring("LANG=C.UTF-8")?,
    ];
    Ok(AndroidPtyCommand {
        executable: cstringPath(&bash)?,
        argv,
        envp,
        cwd: cstringPath(&internalRoot)?,
    })
}

fn androidTerminalDebugInfo(workingDir: &str) -> HostResult<BTreeMap<String, String>> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let rootfsDir = requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?;
    let storageRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_STORAGE_ROOT")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let proot = requiredAndroidRuntimePath("OPERIT_ANDROID_PROOT")?;
    let bash = requiredAndroidRuntimePath("OPERIT_ANDROID_BASH")?;
    let loader = requiredAndroidRuntimePath("OPERIT_ANDROID_LOADER")?;
    let busybox = requiredAndroidRuntimePath("OPERIT_ANDROID_BUSYBOX")?;
    let nativeLibraryDir = requiredAndroidRuntimePath("OPERIT_ANDROID_NATIVE_LIBRARY_DIR")?;
    let rootfsBash = rootfsDir.join("bin/bash");
    let workDir = nonBlank(workingDir, "working_directory")?;
    let mut info = BTreeMap::new();
    insertPathDebug(&mut info, "runtimeDir", &runtimeDir);
    insertPathDebug(&mut info, "rootfsDir", &rootfsDir);
    insertPathDebug(&mut info, "storageRoot", &storageRoot);
    insertPathDebug(&mut info, "internalRoot", &internalRoot);
    insertPathDebug(&mut info, "tmpDir", &tmpDir);
    insertPathDebug(&mut info, "proot", &proot);
    insertPathDebug(&mut info, "bash", &bash);
    insertPathDebug(&mut info, "loader", &loader);
    insertPathDebug(&mut info, "busybox", &busybox);
    insertPathDebug(&mut info, "nativeLibraryDir", &nativeLibraryDir);
    insertPathDebug(&mut info, "rootfsBash", &rootfsBash);
    info.insert("workingDirectory".to_string(), workDir.clone());
    info.insert(
        "argv".to_string(),
        [
            bash.to_string_lossy().to_string(),
            "-c".to_string(),
            "source $HOME/common.sh && start_shell".to_string(),
        ]
        .join(" "),
    );
    Ok(info)
}

fn insertPathDebug(info: &mut BTreeMap<String, String>, key: &str, path: &Path) {
    info.insert(key.to_string(), path.to_string_lossy().to_string());
    info.insert(format!("{key}.exists"), path.exists().to_string());
    info.insert(format!("{key}.isFile"), path.is_file().to_string());
    info.insert(format!("{key}.isDir"), path.is_dir().to_string());
}

fn cstring(value: &str) -> HostResult<CString> {
    CString::new(value).map_err(|error| HostError::new(error.to_string()))
}

fn cstringPath(path: &Path) -> HostResult<CString> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|error| HostError::new(error.to_string()))
}

fn forkPtyExecve(
    command: &AndroidPtyCommand,
    rows: u16,
    cols: u16,
) -> HostResult<(AndroidPid, RawFd)> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = (command, rows, cols);
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut masterFd: libc::c_int = -1;
        let mut termios = operitTermios();
        let mut winsize = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pid =
            unsafe { libc::forkpty(&mut masterFd, ptr::null_mut(), &mut termios, &mut winsize) };
        if pid < 0 {
            androidLogError("forkpty failed");
            return Err(HostError::new("forkpty failed"));
        }
        if pid == 0 {
            unsafe {
                if libc::chdir(command.cwd.as_ptr()) != 0 {
                    libc::write(
                        libc::STDERR_FILENO,
                        b"chdir failed\n".as_ptr().cast(),
                        b"chdir failed\n".len(),
                    );
                    libc::_exit(1);
                }
                let mut argv = command
                    .argv
                    .iter()
                    .map(|item| item.as_ptr())
                    .collect::<Vec<_>>();
                argv.push(ptr::null());
                let mut envp = command
                    .envp
                    .iter()
                    .map(|item| item.as_ptr())
                    .collect::<Vec<_>>();
                envp.push(ptr::null());
                libc::execve(command.executable.as_ptr(), argv.as_ptr(), envp.as_ptr());
                libc::write(
                    libc::STDERR_FILENO,
                    b"execve failed\n".as_ptr().cast(),
                    b"execve failed\n".len(),
                );
                libc::_exit(1);
            }
        }
        Ok((pid, masterFd))
    }
}

#[cfg(target_os = "android")]
fn operitTermios() -> libc::termios {
    let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
    termios.c_iflag = libc::ICRNL | libc::IXON | libc::IXANY;
    termios.c_oflag = libc::OPOST | libc::ONLCR;
    termios.c_lflag = libc::ISIG
        | libc::ICANON
        | libc::ECHO
        | libc::ECHOE
        | libc::ECHOK
        | libc::ECHONL
        | libc::IEXTEN;
    termios.c_cflag = libc::CS8 | libc::CREAD;
    termios.c_cc[libc::VINTR] = b'C' - b'@';
    termios.c_cc[libc::VQUIT] = b'\\' - b'@';
    termios.c_cc[libc::VERASE] = 0x7f;
    termios.c_cc[libc::VKILL] = b'U' - b'@';
    termios.c_cc[libc::VEOF] = b'D' - b'@';
    termios.c_cc[libc::VSTOP] = b'S' - b'@';
    termios.c_cc[libc::VSUSP] = b'Z' - b'@';
    termios.c_cc[libc::VSTART] = b'Q' - b'@';
    termios.c_cc[libc::VMIN] = 1;
    termios.c_cc[libc::VTIME] = 0;
    termios
}

fn readPtyFd(fd: RawFd) -> HostResult<Vec<u8>> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = fd;
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut available: libc::c_int = 0;
        let ioctlResult = unsafe { libc::ioctl(fd, libc::FIONREAD, &mut available) };
        if ioctlResult != 0 {
            androidLogError(&format!("ioctl FIONREAD failed fd={fd}"));
            return Err(HostError::new("ioctl FIONREAD failed for Android PTY"));
        }
        if available <= 0 {
            return Ok(Vec::new());
        }
        let mut output = vec![0u8; available as usize];
        let count = unsafe { libc::read(fd, output.as_mut_ptr().cast(), output.len()) };
        if count < 0 {
            androidLogError(&format!("read failed fd={fd} available={available}"));
            return Err(HostError::new("read failed for Android PTY"));
        }
        output.truncate(count as usize);
        Ok(output)
    }
}

fn writePtyFd(fd: RawFd, data: &[u8]) -> HostResult<usize> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = (fd, data);
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let count = unsafe { libc::write(fd, data.as_ptr().cast(), data.len()) };
        if count < 0 {
            androidLogError(&format!("write failed fd={fd} bytes={}", data.len()));
            return Err(HostError::new("write failed for Android PTY"));
        }
        Ok(count as usize)
    }
}

fn setPtyWindowSize(fd: RawFd, rows: u16, cols: u16) -> HostResult<()> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = (fd, rows, cols);
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut winsize = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &mut winsize) };
        if result != 0 {
            androidLogError(&format!(
                "ioctl TIOCSWINSZ failed fd={fd} rows={rows} cols={cols}"
            ));
            return Err(HostError::new("ioctl TIOCSWINSZ failed for Android PTY"));
        }
        Ok(())
    }
}

fn pollPidExitCode(pid: AndroidPid) -> HostResult<Option<i32>> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = pid;
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut status: libc::c_int = 0;
        let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if result == 0 {
            return Ok(None);
        }
        if result < 0 {
            androidLogError(&format!("waitpid failed pid={pid}"));
            return Err(HostError::new("waitpid failed for Android PTY"));
        }
        if libc::WIFEXITED(status) {
            return Ok(Some(libc::WEXITSTATUS(status)));
        }
        if libc::WIFSIGNALED(status) {
            return Ok(Some(-libc::WTERMSIG(status)));
        }
        Ok(Some(-1))
    }
}

#[derive(Clone, Debug, Default)]
pub struct AndroidWebVisitHost;

impl AndroidWebVisitHost {
    pub fn new() -> Self {
        Self
    }
}

impl WebVisitHost for AndroidWebVisitHost {
    fn visitWeb(&self, _request: WebVisitRequest) -> HostResult<WebVisitResult> {
        Err(HostError::new(
            "Android visit_web requires the Android WebView host bridge",
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct AndroidSystemOperationHost;

impl AndroidSystemOperationHost {
    pub fn new() -> Self {
        Self
    }
}

impl SystemOperationHost for AndroidSystemOperationHost {
    fn toast(&self, message: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android toast requires the Android UI host bridge: {message}"
        )))
    }

    fn sendNotification(&self, title: &str, message: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android notification requires the Android UI host bridge: {title}: {message}"
        )))
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        Err(HostError::new(format!(
            "Android modify_system_setting requires the Android system host bridge: {namespace}/{setting}={value}"
        )))
    }

    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        Err(HostError::new(format!(
            "Android get_system_setting requires the Android system host bridge: {namespace}/{setting}"
        )))
    }

    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android install_app requires the Android package host bridge: {path}"
        )))
    }

    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android uninstall_app requires the Android package host bridge: {packageName}"
        )))
    }

    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData> {
        Err(HostError::new(format!(
            "Android list_installed_apps requires the Android package host bridge, include_system_apps={includeSystemApps}"
        )))
    }

    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android start_app requires the Android package host bridge: {packageName}"
        )))
    }

    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData> {
        Err(HostError::new(format!(
            "Android stop_app requires the Android package host bridge: {packageName}"
        )))
    }

    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData> {
        Err(HostError::new(format!(
            "Android get_notifications requires the Android notification host bridge: limit={limit}, include_ongoing={includeOngoing}"
        )))
    }

    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        Err(HostError::new(format!(
            "Android get_app_usage_time requires the Android usage stats host bridge: package={packageName}, since_hours={sinceHours}, limit={limit}, include_system_apps={includeSystemApps}"
        )))
    }

    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        Err(HostError::new(format!(
            "Android get_device_location requires the Android location host bridge: timeout={timeout}, high_accuracy={highAccuracy}, include_address={includeAddress}"
        )))
    }

    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        Err(HostError::new(
            "Android get_device_info requires the Android device info host bridge",
        ))
    }
}
