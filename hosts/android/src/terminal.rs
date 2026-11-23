use operit_host_api::terminalSessionKey as sessionKey;
use operit_host_api::TerminalProfiles::{profileMounts, TerminalMount};
use operit_host_api::TerminalControl::{controlToSequence, normalizeControl};
use operit_host_api::TerminalText::{appendPtyOutput as appendAndroidPtyOutput, dropCommandEcho, nonBlank, renderTerminalText, rfindBytes, stripPtyDeviceStatusReports, stripPtyPromptMarkers, scanPtyOutput, updatePtyCursor as updateAndroidPtyCursor, PtyCursor as AndroidPtyCursor};
use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::ffi::CString;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use uuid::Uuid;

use crate::runtime_common::requiredAndroidRuntimePath;

#[cfg(target_os = "android")]
use std::ptr;

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);
const PTY_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
const PTY_PROMPT_MARKER_PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
const PTY_PROMPT_MARKER_END: u8 = 7;
const PLATFORM: &str = "android";
const PROOT_TERMINAL: &str = "proot";
const CHROOT_TERMINAL: &str = "chroot";
const ANDROID_SYSTEM_TERMINAL: &str = "android-system";
const PRIMARY_TERMINAL_TYPE: &str = "bash";

fn resolveAndroidNativeWorkingDirectory(
    path: &str,
    documentOperation: &dyn Fn(serde_json::Value) -> HostResult<serde_json::Value>,
) -> HostResult<String> {
    let Some(resource) = operit_host_api::FileSystemResource::FileSystemResource::parse(path)? else {
        return Ok(path.to_string());
    };
    if resource.backend != "android_documents" {
        return Err(HostError::new("Unsupported Android terminal filesystem backend"));
    }
    let value = documentOperation(serde_json::json!({
        "operation": "resolveNativeDirectory", "path": path,
    }))?;
    let directory = value.as_str().filter(|value| value.starts_with('/') && !value.contains('\0'))
        .ok_or_else(|| HostError::new("Android document provider returned an invalid native directory"))?;
    Ok(directory.to_string())
}
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

#[derive(Clone, Default)]
pub struct AndroidTerminalHost {
    state: Arc<Mutex<AndroidTerminalState>>,
    profile: Option<operit_host_api::TerminalProfile>,
}

#[derive(Default)]
struct AndroidTerminalState {
    sessionNameToId: BTreeMap<String, String>,
    hiddenExecutorKeyToSessionId: BTreeMap<String, String>,
    ptySessions: BTreeMap<String, AndroidPtySession>,
}

struct AndroidPtySession {
    sessionName: String,
    terminal: String,
    terminalType: String,
    workingDir: String,
    pid: AndroidPtyProcess,
    masterFd: RawFd,
    writer: Arc<Mutex<RawFd>>,
    reader: Option<Box<dyn FnOnce() + Send>>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: AndroidPtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    commandRunning: bool,
    exitCode: Option<i32>,
}

type AndroidPtyCommandOutput = Arc<operit_host_api::TerminalCommand::TerminalCommandBuffer>;

/// Keeps remote process ownership distinct from local child PIDs.
#[derive(Clone, Copy, Debug)]
enum AndroidPtyProcess {
    Local(AndroidPid),
    Shizuku(i32),
}

type AndroidPtyCursorState = Arc<Mutex<AndroidPtyCursor>>;

impl Drop for AndroidPtySession {
    /// Stops the reader before releasing its separately owned write handle.
    fn drop(&mut self) {
        self.commandOutput.fail(HostError::new("Terminal session was closed"));
        if let Some(close) = self.reader.take() { close(); }
        #[cfg(target_os = "android")]
        unsafe {
            terminateAndroidPtyProcess(self.pid);
            let mut writerFd = self
                .writer
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let fd = *writerFd;
            *writerFd = -1;
            if fd >= 0 {
                libc::close(fd);
            }
        }
    }
}

impl AndroidTerminalHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts an Android PTY session for one exact terminal implementation and type.
    pub fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let normalizedTerminalType = requireAndroidTerminal(terminal, terminalType)?;
        if self.implementation(&normalizedTerminalType)? != terminal.trim() {
            return Err(HostError::new("Android terminal requires a matching launch profile"));
        }
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createAndroidPtySession(
            self.profile.as_ref(),
            normalizedSessionName,
            normalizedTerminalType,
            workDir,
            rows,
            cols,
        )?;
        let sessionId = nextTerminalId();
        let mut state = self.lockState()?;
        state.ptySessions.insert(sessionId.clone(), session);
        Ok(sessionId)
    }

    /// Resolves an exact implementation without conflating two Bash launchers.
    fn implementation(&self, terminalType: &str) -> HostResult<&str> {
        if let Some(profile) = &self.profile {
            requireAndroidTerminal(&profile.terminal, terminalType)?;
            return Ok(&profile.terminal);
        }
        androidTerminalName(terminalType)
    }

    pub fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        let mut state = self.lockState()?;
        let session = state.ptySessions.get_mut(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        let mut output = session
            .output
            .lock()
            .map_err(|_| HostError::new("android pty output mutex poisoned"))?;
        Ok(output.drain(..).collect())
    }

    pub fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        let state = self.lockState()?;
        let session = state.ptySessions.get(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        writeAndroidPtyBytes(&session.writer, data)
    }

    pub fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        let state = self.lockState()?;
        let session = state.ptySessions.get(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        resizeAndroidPtyProcess(session.pid, session.masterFd, rows, cols)
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

    /// Removes session mappings before tearing down its process outside the registry lock.
    pub fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        let removed = {
            let mut state = self.lockState()?;
            let removed = state.ptySessions.remove(sessionId).ok_or_else(|| {
                HostError::new(format!("Android PTY session does not exist: {sessionId}"))
            })?;
            removeAndroidSessionMappings(&mut state, sessionId);
            removed
        };
        // Process teardown may block and must not hold the terminal registry lock.
        drop(removed);
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
    /// Offers ARM64 rootfs archives only on Android devices executing that architecture.
    fn terminalDistributions(&self) -> HostResult<Vec<operit_host_api::TerminalDistribution>> {
        if std::env::consts::ARCH != "aarch64" {
            return Ok(Vec::new());
        }
        Ok(operit_host_native_common::TerminalRootfs::distributions())
    }

    /// Installs each downloaded distribution into its own device-local rootfs directory.
    fn installTerminalDistribution(
        &self,
        distributionId: &str,
        instanceId: &str,
        name: &str,
        control: operit_host_api::HttpDownloadControl,
        progress: operit_host_api::TerminalInstallProgressCallback,
    ) -> HostResult<operit_host_api::TerminalProfile> {
        if std::env::consts::ARCH != "aarch64" {
            return Err(HostError::new(
                "Downloadable rootfs images require an ARM64 Android host",
            ));
        }
        let runtime = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_ROOT")?;
        let relative = operit_host_api::RuntimeStorageLayout::TERMINAL_ROOTFS_DIR_PATH
            .strip_prefix("runtime/")
            .ok_or_else(|| HostError::new("Invalid terminal storage definition"))?;
        operit_host_native_common::TerminalRootfs::install(
            &runtime.join(relative),
            distributionId,
            instanceId,
            name,
            PROOT_TERMINAL,
            "bash",
            control,
            progress,
        )
    }

    /// Removes only a fresh host-owned instance when durable profile registration fails.
    fn discardTerminalDistribution(&self, instanceId: &str) -> HostResult<()> {
        let runtime = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_ROOT")?;
        let relative = operit_host_api::RuntimeStorageLayout::TERMINAL_ROOTFS_DIR_PATH
            .strip_prefix("runtime/")
            .ok_or_else(|| HostError::new("Invalid terminal storage definition"))?;
        operit_host_native_common::TerminalRootfs::discard(&runtime.join(relative), instanceId)
    }

    /// Registers user-supplied Linux root filesystems without modifying the bundled runtime.
    fn terminalProfileSupport(&self) -> HostResult<Vec<operit_host_api::TerminalProfileSupport>> {
        Ok([PROOT_TERMINAL, CHROOT_TERMINAL, ANDROID_SYSTEM_TERMINAL]
            .into_iter().map(|terminal| operit_host_api::TerminalProfileSupport {
                fields: if terminal == ANDROID_SYSTEM_TERMINAL {
                    Vec::new()
                } else {
                    vec!["rootfs".into(), "mounts".into()]
                },
                terminal: terminal.into(),
                terminalType: if terminal == ANDROID_SYSTEM_TERMINAL { "shell" } else { "bash" }.into(),
                fieldChoices: None,
            })
            .collect())
    }

    /// Creates a separate PTY registry for one Linux rootfs and mount configuration.
    fn configuredTerminal(
        &self,
        profile: &operit_host_api::TerminalProfile,
    ) -> HostResult<Arc<dyn TerminalHost>> {
        requireAndroidTerminal(&profile.terminal, &profile.terminalType)?;
        androidProfileRootfs(profile)?;
        androidProfileMounts(profile)?;
        Ok(Arc::new(Self {
            state: Arc::default(),
            profile: Some(profile.clone()),
        }))
    }

    fn resolveNativeWorkingDirectory(&self, path: &str) -> HostResult<String> {
        resolveAndroidNativeWorkingDirectory(path, &crate::document_filesystem::call)
    }

    /// Resolves proot guest paths separately from Android system-shell host paths.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        match requireAndroidTerminal(terminal, terminalType)?.as_str() {
            "bash" => operit_host_api::TerminalWorkingDirectory::resolveLinuxGuestDirectory(
                workingDir,
                resolveHostDirectory,
            ),
            "shell" => {
                let directory = resolveHostDirectory(workingDir)?;
                let transport = crate::terminal_privilege::privileges()?.select()?;
                let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
                Ok(androidSystemShellWorkingDirectory(
                    &transport,
                    &directory,
                    &internalRoot,
                ))
            }
            value => Err(HostError::new(format!(
                "Unsupported Android terminal type: {value}"
            ))),
        }
    }

    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        if let Some(profile) = &self.profile {
            return Ok(TerminalInfo {
                platform: PLATFORM.into(),
                terminal: profile.terminal.clone(),
                terminalType: profile.terminalType.clone(),
                types: vec![TerminalTypeInfo {
                    terminal: profile.terminal.clone(),
                    terminalType: profile.terminalType.clone(),
                    available: true,
                    description: profile.name.clone(),
                    nameKey: profile.nameKey.clone(),
                }],
            });
        }
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: PROOT_TERMINAL.to_string(),
            terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
            types: vec![
                TerminalTypeInfo {
                    terminal: PROOT_TERMINAL.to_string(),
                    terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
                    available: true,
                    description: "Android proot Linux bash terminal".to_string(),
                    nameKey: Some("terminalBuiltinAndroidLinux".into()),
                },
                TerminalTypeInfo {
                    terminal: ANDROID_SYSTEM_TERMINAL.to_string(),
                    terminalType: "shell".to_string(),
                    available: true,
                    description: "Android system shell terminal".to_string(),
                    nameKey: Some("terminalBuiltinAndroidSystem".into()),
                },
            ],
        })
    }

    fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        AndroidTerminalHost::startPtySession(
            self,
            sessionName,
            terminal,
            terminalType,
            workingDir,
            rows,
            cols,
        )
    }

    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        AndroidTerminalHost::readPtySession(self, sessionId)
    }

    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        AndroidTerminalHost::writePtySession(self, sessionId, data)
    }

    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        AndroidTerminalHost::resizePtySession(self, sessionId, rows, cols)
    }

    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        AndroidTerminalHost::pollPtyExitCode(self, sessionId)
    }

    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        AndroidTerminalHost::closePtySession(self, sessionId)
    }

    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let mut state = self.lockState()?;
        let mut entries = Vec::new();
        for (sessionId, session) in state.ptySessions.iter_mut() {
            if session.exitCode.is_none() {
                if let Some(exitCode) = pollPidExitCode(session.pid)? {
                    session.exitCode = Some(exitCode);
                }
            }
            if session.exitCode.is_some() {
                continue;
            }
            entries.push(TerminalSessionListEntry {
                sessionId: sessionId.clone(),
                sessionName: session.sessionName.clone(),
                platform: PLATFORM.to_string(),
                terminal: session.terminal.clone(),
                terminalType: session.terminalType.clone(),
                sessionKind: "pty".to_string(),
                workingDir: session.workingDir.clone(),
                commandRunning: session.commandRunning,
            });
        }
        Ok(entries)
    }

    /// Creates or reuses a named session in the configured default terminal.
    fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTypedSession(sessionName, PRIMARY_TERMINAL_TYPE)
    }

    /// Creates or reuses a named session of exactly the requested interpreter type.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let normalizedTerminalType = nonBlank(terminalType, "type")?;
        let terminal = self.implementation(&normalizedTerminalType)?;
        let key = sessionKey(&normalizedTerminalType, &normalizedSessionName);
        {
            let mut state = self.lockState()?;
            if let Some(sessionId) = state.sessionNameToId.get(&key).cloned() {
                if state.ptySessions.contains_key(&sessionId) {
                    return Ok(TerminalSessionInfo {
                        sessionId,
                        sessionName: normalizedSessionName,
                        platform: PLATFORM.to_string(),
                        terminal: terminal.to_string(),
                        terminalType: normalizedTerminalType,
                        isNewSession: false,
                    });
                }
                state.sessionNameToId.remove(&key);
            }
        }

        let workingDir = initialAndroidWorkingDir(&normalizedTerminalType)?;
        let session = createAndroidPtySession(
            self.profile.as_ref(),
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
            workingDir,
            24,
            80,
        )?;
        let sessionId = nextTerminalId();
        let mut state = self.lockState()?;
        state.sessionNameToId.insert(key, sessionId.clone());
        state.ptySessions.insert(sessionId.clone(), session);
        Ok(TerminalSessionInfo {
            sessionId,
            sessionName: normalizedSessionName,
            platform: PLATFORM.to_string(),
            terminal: terminal.to_string(),
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
        self.executeInSessionStreaming(sessionId, command, timeoutMs, &mut |_| Ok(()))
    }

    /// Runs explicit synchronous callers through the same event-driven command state.
    fn executeInSessionStreaming(
        &self, sessionId: &str, command: &str, timeoutMs: u64,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        operit_host_native_common::runTerminalCommandSynchronously(
            Box::new(self.beginCommand(sessionId, command, timeoutMs)?), onOutput,
        )
    }

    /// Returns command state whose output waits never occupy a host worker.
    fn beginInSessionStreaming(self: Arc<Self>, sessionId: String, command: String, timeoutMs: u64)
        -> HostResult<Box<dyn operit_host_api::TerminalCommand::TerminalCommandExecution>> {
        Ok(Box::new(self.beginCommand(&sessionId, &command, timeoutMs)?))
    }



    fn executeHiddenCommand(
        &self,
        command: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let normalizedCommand = nonBlank(command, "command")?;
        let normalizedTerminalType = PRIMARY_TERMINAL_TYPE.to_string();
        let normalizedExecutorKey = match executorKey.trim() {
            "" => "default".to_string(),
            value => value.to_string(),
        };
        let key = sessionKey(&normalizedTerminalType, &normalizedExecutorKey);
        let sessionId =
            hiddenAndroidPtySessionId(self, &key, &normalizedExecutorKey, &normalizedTerminalType)?;
        let output = self.executeInSession(&sessionId, &normalizedCommand, timeoutMs)?;
        Ok(HiddenTerminalCommandOutput {
            command: normalizedCommand,
            output: output.output,
            exitCode: output.exitCode,
            executorKey: normalizedExecutorKey,
            platform: PLATFORM.to_string(),
            terminal: output.terminal,
            terminalType: normalizedTerminalType,
            timedOut: output.timedOut,
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
            .ptySessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let acceptedChars =
            applyAndroidPtyTerminalInput(session, input, control.and_then(normalizeControl))?;
        Ok(TerminalInputOutput {
            sessionId: normalizedSessionId,
            acceptedChars,
        })
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        self.closePtySession(&normalizedSessionId)?;
        Ok(TerminalCloseOutput {
            sessionId: normalizedSessionId.clone(),
            success: true,
            message: format!("Terminal session closed: {normalizedSessionId}"),
        })
    }

    fn closeSessionIfIdle(&self, sessionId: &str, sessionName: &str) -> HostResult<TerminalCloseOutput> {
        let id = nonBlank(sessionId, "session_id")?;
        let name = nonBlank(sessionName, "session_name")?;
        let mut state = self.lockState()?;
        let session = state.ptySessions.get(&id)
            .ok_or_else(|| HostError::new(format!("Terminal session does not exist: {id}")))?;
        if !operit_host_api::terminalSessionNameMatches(&session.sessionName, &name) {
            return Err(HostError::new("Terminal session name does not match"));
        }
        if session.commandRunning {
            return Err(HostError::new("Terminal session has a command in progress"));
        }
        let removed = state.ptySessions.remove(&id);
        state.sessionNameToId.retain(|_, value| value != &id);
        state.hiddenExecutorKeyToSessionId.retain(|_, value| value != &id);
        drop(state);
        drop(removed);
        Ok(TerminalCloseOutput { sessionId: id.clone(), success: true, message: format!("Idle terminal session closed: {id}") })
    }

    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let mut state = self.lockState()?;
        let session = state
            .ptySessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let content = androidPtyScreenContent(session)?;
        let rows = content.lines().count();
        let cols = content
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        Ok(TerminalScreenOutput {
            sessionId: normalizedSessionId,
            platform: PLATFORM.to_string(),
            terminal: session.terminal.clone(),
            terminalType: session.terminalType.clone(),
            rows,
            cols,
            content,
            commandRunning: session.commandRunning,
        })
    }
}


/// Owns command parsing independently of the executor thread awaiting terminal events.
struct CooperativeCommand {
    host: AndroidTerminalHost,
    sessionId: String,
    command: String,
    terminal: String,
    terminalType: String,
    commandEcho: String,
    commandOutput: AndroidPtyCommandOutput,
    timeoutMs: u64,
    collected: Vec<u8>,
    workingDir: Option<String>,
    writer: Arc<Mutex<RawFd>>,
}

impl AndroidTerminalHost {
    /// Submits one command and reserves its session before asynchronous observation begins.
    fn beginCommand(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<CooperativeCommand> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let normalizedCommand = nonBlank(command, "command")?;
        let commandLine = crate::terminal_command::posixCommandLine(&normalizedCommand);
        let (terminal, terminalType, commandOutput, writer) = {
            let mut state = self.lockState()?;
            let session = state
                .ptySessions
                .get_mut(&normalizedSessionId)
                .ok_or_else(|| {
                    HostError::new(format!("Terminal session does not exist: {sessionId}"))
                })?;
            if session.commandRunning {
                return Err(HostError::new(
                    "Terminal session already has a command in progress",
                ));
            }
            clearAndroidPtyCommandOutput(&session.commandOutput)?;
            let commandInput = format!("{commandLine}\r");
            writeAndroidPtyBytes(&session.writer, commandInput.as_bytes())?;
            session.commandRunning = true;
            (
                session.terminal.clone(),
                session.terminalType.clone(),
                session.commandOutput.clone(),
                session.writer.clone(),
            )
        };
        Ok(CooperativeCommand {
            host: self.clone(),
            commandEcho: commandLine,
            sessionId: normalizedSessionId,
            command: normalizedCommand,
            terminal,
            terminalType,
            commandOutput,
            timeoutMs,
            collected: Vec::new(),
            workingDir: None,
            writer,
        })
    }
}

impl operit_host_api::TerminalCommand::TerminalCommandExecution for CooperativeCommand {
    /// Registers a wakeup and parses available protocol records without waiting.
    fn pollOutput(
        &mut self,
        context: &mut std::task::Context<'_>,
        expired: bool,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> std::task::Poll<HostResult<TerminalCommandOutput>> {
        let result = (|| -> HostResult<Option<TerminalCommandOutput>> {
            let previousLength = self.collected.len();
            let readerError = self.commandOutput.pollDrain(context, &mut self.collected)?;
            if self.collected.len() > previousLength {
                onOutput(&self.collected[previousLength..])?;
            }
            if self.timeoutMs == 0 && self.collected.len() > PTY_OUTPUT_LIMIT {
                self.collected
                    .drain(..self.collected.len() - PTY_OUTPUT_LIMIT);
            }
            let (output, exitCode, timedOut) =
                if let Some(marker) = findLastAndroidPtyPromptMarker(&self.collected)? {
                    let output = androidPtyCompletedCommandOutputText(
                        &self.collected,
                        &self.commandEcho,
                        marker.start,
                    );
                    self.workingDir = Some(marker.workingDir);
                    (output, marker.exitCode, false)
                } else {
                    if let Some(error) = readerError {
                        return Err(error);
                    }
                    if !expired {
                        return Ok(None);
                    }
                    (
                        androidPtyTimedOutCommandOutputText(&self.collected, &self.commandEcho),
                        -1,
                        true,
                    )
                };
            Ok(Some(TerminalCommandOutput {
                command: self.command.clone(),
                output,
                exitCode,
                sessionId: self.sessionId.clone(),
                platform: PLATFORM.to_string(),
                terminal: self.terminal.clone(),
                terminalType: self.terminalType.clone(),
                timedOut,
            }))
        })();
        match result {
            Ok(Some(receipt)) => std::task::Poll::Ready(Ok(receipt)),
            Ok(None) => std::task::Poll::Pending,
            Err(error) => std::task::Poll::Ready(Err(error)),
        }
    }

    /// Keeps execution lifetime independent from the foreground observation deadline.
    fn timeoutMs(&self) -> u64 {
        self.timeoutMs
    }

    /// Sends Ctrl+C on the finite OS boundary without retaining the session registry lock.
    fn beginInterrupt(&mut self) -> HostResult<()> {
        self.commandOutput.clear()?;
        self.collected.clear();
        writeAndroidPtyBytes(&self.writer, b"\x03")?;
        Ok(())
    }

    /// Awaits the shell readiness marker through output notifications, including silent close.
    fn pollInterrupt(
        &mut self,
        context: &mut std::task::Context<'_>,
        expired: bool,
    ) -> std::task::Poll<HostResult<()>> {
        let result = (|| -> HostResult<Option<()>> {
            let readerError = self.commandOutput.pollDrain(context, &mut self.collected)?;
            if let Some(marker) = findLastAndroidPtyPromptMarker(&self.collected)? {
                self.workingDir = Some(marker.workingDir);
                return Ok(Some(()));
            }
            if let Some(error) = readerError {
                return Err(error);
            }
            if expired {
                return Err(HostError::new(
                    "Timed out waiting for the interrupted terminal to become ready",
                ));
            }
            Ok(None)
        })();
        match result {
            Ok(Some(())) => std::task::Poll::Ready(Ok(())),
            Ok(None) => std::task::Poll::Pending,
            Err(error) => std::task::Poll::Ready(Err(error)),
        }
    }

    /// Releases the busy flag only after completion, or closes an unresponsive session.
    fn finish(
        self: Box<Self>,
        result: HostResult<TerminalCommandOutput>,
        settled: HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        let mut state = self.host.lockState()?;
        if let Err(error) = settled {
            removeAndroidSessionMappings(&mut state, &self.sessionId);
            let removed = state.ptySessions.remove(&self.sessionId);
            drop(state);
            drop(removed);
            return Err(HostError::new(format!(
                "Terminal session {} closed because its command could not be interrupted: {}",
                self.sessionId, error.message
            )));
        }
        if let Some(session) = state.ptySessions.get_mut(&self.sessionId) {
            session.commandRunning = false;
            if let Some(workingDir) = &self.workingDir {
                session.workingDir = workingDir.clone();
            }
        }
        result
    }
}


fn hiddenAndroidPtySessionId(
    host: &AndroidTerminalHost,
    executorKey: &str,
    executorLabel: &str,
    terminalType: &str,
) -> HostResult<String> {
    {
        let mut state = host.lockState()?;
        if let Some(sessionId) = state.hiddenExecutorKeyToSessionId.get(executorKey).cloned() {
            if state.ptySessions.contains_key(&sessionId) {
                return Ok(sessionId);
            }
            state.hiddenExecutorKeyToSessionId.remove(executorKey);
            removeAndroidSessionMappings(&mut state, &sessionId);
        }
    }

    let session = createAndroidPtySession(
        host.profile.as_ref(),
        format!("hidden:{executorLabel}"),
        terminalType.to_string(),
        initialAndroidWorkingDir(terminalType)?,
        24,
        80,
    )?;
    let sessionId = nextTerminalId();
    let mut state = host.lockState()?;
    state
        .hiddenExecutorKeyToSessionId
        .insert(executorKey.to_string(), sessionId.clone());
    state.ptySessions.insert(sessionId.clone(), session);
    Ok(sessionId)
}

fn createAndroidPtySession(
    profile: Option<&operit_host_api::TerminalProfile>,
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
) -> HostResult<AndroidPtySession> {
    androidLogInfo(&format!(
        "createAndroidPtySession start name={sessionName} type={terminalType} workingDir={workingDir} size={rows}x{cols}"
    ));
    let command = match profile {
        Some(profile) => buildAndroidProfileCommand(profile, &workingDir)?,
        None => buildAndroidPtyCommand(&terminalType, &workingDir)?,
    };
    let (pid, readerFd, masterFd) = startAndroidPtyProcess(&command, rows, cols)?;
    androidLogInfo(&format!(
        "createAndroidPtySession forked pid={pid:?} masterFd={masterFd} name={sessionName}"
    ));
    let writer = Arc::new(Mutex::new(masterFd));
    let output = Arc::new(Mutex::new(VecDeque::new()));
    let commandOutput: AndroidPtyCommandOutput =
        Arc::new(operit_host_api::TerminalCommand::TerminalCommandBuffer::new(PTY_OUTPUT_LIMIT));
    let screenOutput = Arc::new(Mutex::new(VecDeque::new()));
    let cursorState: AndroidPtyCursorState =
        Arc::new(Mutex::new(AndroidPtyCursor { row: 1, col: 1 }));
    let mut session = AndroidPtySession {
        sessionName,
        terminal: match profile {
            Some(profile) => profile.terminal.clone(),
            None => androidTerminalName(&terminalType)?.into(),
        },
        terminalType,
        workingDir: command.workingDir.unwrap_or(workingDir),
        pid,
        masterFd,
        writer: writer.clone(),
        reader: None,
        output: output.clone(),
        commandOutput: commandOutput.clone(),
        screenOutput: screenOutput.clone(),
        commandRunning: false,
        exitCode: None,
    };
    session.reader = Some(scheduleAndroidPtyReader(
        readerFd,
        writer.clone(),
        output.clone(),
        commandOutput.clone(),
        screenOutput.clone(),
        cursorState,
    )?);
    if let Some(input) = command.startupInput {
        writeAndroidPtyBytes(&writer, input.as_bytes())?;
    }
    if let Err(error) =
        operit_host_native_common::waitForTerminalPrompt(
        &commandOutput,
        10000,
        |bytes| findLastAndroidPtyPromptMarker(bytes).map(|marker| marker.is_some()),
        androidPtyStartupError,
    )
    {
        androidLogError(&format!(
            "createAndroidPtySession initial prompt failed pid={pid:?} masterFd={masterFd} name={}: {error}", session.sessionName
        ));
        return Err(error);
    }
    clearAndroidPtyCommandOutput(&commandOutput)?;
    androidLogInfo(&format!(
        "createAndroidPtySession ready pid={pid:?} masterFd={masterFd} name={}",
        session.sessionName
    ));
    Ok(session)
}

/// Registers terminal read readiness without holding a service worker or polling a timer.
fn scheduleAndroidPtyReader(
    readerFd: RawFd, writer: Arc<Mutex<RawFd>>, output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: AndroidPtyCommandOutput, screenOutput: Arc<Mutex<VecDeque<u8>>>,
    cursorState: AndroidPtyCursorState,
) -> HostResult<Box<dyn FnOnce() + Send>> {
    #[cfg(target_os = "android")]
    {
        use std::os::fd::{FromRawFd, OwnedFd};
        let descriptor = unsafe { OwnedFd::from_raw_fd(readerFd) };
        {
            // Shizuku transports use separate read/write pipes, so configure the writer explicitly.
            let fd = writer.lock().map_err(|_| HostError::new("Android PTY writer state poisoned"))?;
            let flags = unsafe { libc::fcntl(*fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(*fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        let closed = commandOutput.clone();
        let reader = operit_host_native_common::scheduleNativeFdReader("android-pty-reader", descriptor,
            move |data| {
                processAndroidPtyTerminalQueries(&writer, &cursorState, data)?;
                let commandChunk = stripPtyDeviceStatusReports(data);
                let visibleChunk = stripPtyPromptMarkers(&commandChunk);
                appendAndroidPtyOutput(&output, &visibleChunk, PTY_OUTPUT_LIMIT);
                appendAndroidPtyOutput(&screenOutput, &visibleChunk, PTY_OUTPUT_LIMIT);
                appendAndroidPtyCommandOutput(&commandOutput, &commandChunk);
                Ok(())
            },
            move |error| closed.fail(HostError::new(format!("Android PTY reached EOF or stopped: {}", error.message))),
        )?;
        Ok(Box::new(move || {
            if let Err(error) = reader.close() { androidLogError(&error.message); }
        }))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (readerFd, writer, output, commandOutput, screenOutput, cursorState);
        Err(HostError::new("Android PTY is only available on Android target"))
    }
}

/// Uses the session-owned event buffer for command output.
fn appendAndroidPtyCommandOutput(output: &AndroidPtyCommandOutput, data: &[u8]) { output.append(data); }

/// Answers terminal cursor queries on the finite OS boundary without blocking output observation.
fn processAndroidPtyTerminalQueries(
    writer: &Arc<Mutex<RawFd>>,
    cursorState: &AndroidPtyCursorState,
    data: &[u8],
) -> HostResult<()> {
    scanPtyOutput(data, || {
        let response = {
            let cursor = cursorState.lock().map_err(|_| HostError::new("PTY cursor state poisoned"))?;
            format!("\x1b[{};{}R", cursor.row, cursor.col)
        };
        let writer = writer.clone();
        operit_host_native_common::scheduleNativeBlockingTask("terminal-cursor-response", move || {
            writeAndroidPtyBytes(&writer, response.as_bytes()).map(|_| ())
        })?;
        Ok(())
    }, |value| updateAndroidPtyCursor(cursorState, value))
}

/// Uses the session-owned event buffer for command output.
fn clearAndroidPtyCommandOutput(output: &AndroidPtyCommandOutput) -> HostResult<()> { output.clear() }


/// Preserves startup diagnostics in the error returned to terminal callers and tools.
fn androidPtyStartupError(data: &[u8]) -> HostError {
    let visibleBytes = stripPtyPromptMarkers(data);
    let visibleOutput = renderTerminalText(&String::from_utf8_lossy(&visibleBytes));
    HostError::new(format!(
        "Timed out waiting for Android terminal prompt\nStartup output:\n{}",
        visibleOutput.trim(),
    ))
}


#[derive(Clone, Debug)]
struct AndroidPtyPromptMarker {
    start: usize,
    workingDir: String,
    exitCode: i32,
}

fn findLastAndroidPtyPromptMarker(data: &[u8]) -> HostResult<Option<AndroidPtyPromptMarker>> {
    let Some(start) = rfindBytes(data, PTY_PROMPT_MARKER_PREFIX) else {
        return Ok(None);
    };
    let payloadStart = start + PTY_PROMPT_MARKER_PREFIX.len();
    let Some(relativeEnd) = data[payloadStart..]
        .iter()
        .position(|value| *value == PTY_PROMPT_MARKER_END)
    else {
        return Ok(None);
    };
    let payloadEnd = payloadStart + relativeEnd;
    let payload = std::str::from_utf8(&data[payloadStart..payloadEnd]).map_err(|error| {
        HostError::new(format!("Invalid Android PTY prompt marker UTF-8: {error}"))
    })?;
    let (workingDirText, exitCodeText) = payload
        .split_once(':')
        .ok_or_else(|| HostError::new(format!("Invalid Android PTY prompt marker '{payload}'")))?;
    let workingDirBytes = BASE64_STANDARD
        .decode(workingDirText.as_bytes())
        .map_err(|error| {
            HostError::new(format!("Invalid Android PTY prompt cwd marker: {error}"))
        })?;
    let workingDir = String::from_utf8(workingDirBytes).map_err(|error| {
        HostError::new(format!("Invalid Android PTY prompt cwd UTF-8: {error}"))
    })?;
    let exitCode = parseAndroidExitCode(exitCodeText)?;
    Ok(Some(AndroidPtyPromptMarker {
        start,
        workingDir,
        exitCode,
    }))
}

fn androidPtyCompletedCommandOutputText(raw: &[u8], command: &str, promptStart: usize) -> String {
    let visibleBytes = stripPtyPromptMarkers(&raw[..promptStart]);
    let text = String::from_utf8_lossy(&visibleBytes);
    let clean = renderTerminalText(&text);
    dropCommandEcho(clean, command)
}

fn androidPtyTimedOutCommandOutputText(raw: &[u8], command: &str) -> String {
    let visibleBytes = stripPtyPromptMarkers(raw);
    let text = String::from_utf8_lossy(&visibleBytes);
    let clean = renderTerminalText(&text);
    dropCommandEcho(clean, command)
}

fn androidPtyScreenContent(session: &AndroidPtySession) -> HostResult<String> {
    let buffer = session
        .screenOutput
        .lock()
        .map_err(|_| HostError::new("android pty screen output mutex poisoned"))?;
    let bytes = buffer.iter().copied().collect::<Vec<_>>();
    let visibleBytes = stripPtyPromptMarkers(&bytes);
    let text = String::from_utf8_lossy(&visibleBytes);
    Ok(renderTerminalText(&text))
}

fn parseAndroidExitCode(exitCodeText: &str) -> HostResult<i32> {
    exitCodeText.trim().parse::<i32>().map_err(|error| {
        HostError::new(format!(
            "Invalid terminal exit code marker '{exitCodeText}': {error}"
        ))
    })
}

fn writeAndroidPtyBytes(writer: &Arc<Mutex<RawFd>>, data: &[u8]) -> HostResult<usize> {
    let fd = writer
        .lock()
        .map_err(|_| HostError::new("android pty writer mutex poisoned"))?;
    writePtyFd(*fd, data)
}

fn applyAndroidPtyTerminalInput(
    session: &mut AndroidPtySession,
    input: Option<&str>,
    control: Option<&str>,
) -> HostResult<usize> {
    let mut acceptedChars = 0;
    if let Some(input) = input {
        writeAndroidPtyBytes(&session.writer, input.as_bytes())?;
        acceptedChars += input.chars().count();
    }
    if let Some(control) = control {
        let sequence = match control {
            "enter" => "\r".to_string(),
            value => controlToSequence(value, input)?,
        };
        writeAndroidPtyBytes(&session.writer, sequence.as_bytes())?;
        acceptedChars += sequence.chars().count();
    }
    Ok(acceptedChars)
}

/// Normalizes Android terminal type names accepted by the host.
fn normalizeAndroidTerminalType(terminalType: &str) -> HostResult<String> {
    match terminalType.trim() {
        "bash" => Ok("bash".to_string()),
        "shell" => Ok("shell".to_string()),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for android host: {value}"
        ))),
    }
}

/// Validates one Android terminal implementation and shell-type pair.
fn requireAndroidTerminal(terminal: &str, terminalType: &str) -> HostResult<String> {
    let normalizedTerminalType = normalizeAndroidTerminalType(terminalType)?;
    match (terminal.trim(), normalizedTerminalType.as_str()) {
        (PROOT_TERMINAL | CHROOT_TERMINAL, "bash") | (ANDROID_SYSTEM_TERMINAL, "shell") => Ok(normalizedTerminalType),
        (implementation, terminalType) => Err(HostError::new(format!(
            "Unsupported Android terminal implementation and type: {implementation}/{terminalType}"
        ))),
    }
}

/// Returns the Android terminal implementation registered for one shell semantic type.
fn androidTerminalName(terminalType: &str) -> HostResult<&'static str> {
    match terminalType {
        "bash" => Ok(PROOT_TERMINAL),
        "shell" => Ok(ANDROID_SYSTEM_TERMINAL),
        value => Err(HostError::new(format!(
            "Unsupported Android terminal type: {value}"
        ))),
    }
}

/// Returns the initial working directory for an Android terminal type.
fn initialAndroidWorkingDir(terminalType: &str) -> HostResult<String> {
    match terminalType {
        "bash" => Ok("/root".to_string()),
        "shell" => requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")
            .map(|path| path.to_string_lossy().to_string()),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for android host: {value}"
        ))),
    }
}

#[cfg(target_os = "android")]
fn androidLogInfo(message: &str) {
    androidLogWrite(4, message);
}

#[cfg(not(target_os = "android"))]
fn androidLogInfo(_message: &str) {}

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

fn removeAndroidSessionMappings(state: &mut AndroidTerminalState, sessionId: &str) {
    state.sessionNameToId.retain(|_, value| value != sessionId);
    state
        .hiddenExecutorKeyToSessionId
        .retain(|_, value| value != sessionId);
}

fn nextTerminalId() -> String {
    Uuid::new_v4().to_string()
}

struct AndroidPtyCommand {
    executable: CString,
    argv: Vec<CString>,
    envp: Vec<CString>,
    cwd: CString,
    shizuku: bool,
    dropRootToShell: bool,
    workingDir: Option<String>,
    startupInput: Option<String>,
}

/// Validates the selected rootfs and keeps the host filesystem itself out of the launcher.
fn androidProfileRootfs(profile: &operit_host_api::TerminalProfile) -> HostResult<std::path::PathBuf> {
    let rootfs = match (profile.terminal.as_str(), profile.options.get("rootfs")) {
        (PROOT_TERMINAL | CHROOT_TERMINAL, Some(path)) if Path::new(path).is_absolute() => {
            Path::new(path).canonicalize()?
        }
        (PROOT_TERMINAL, None) => requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?.canonicalize()?,
        _ => return Err(HostError::new("Android chroot requires an absolute rootfs directory")),
    };
    if rootfs.parent().is_none() || !rootfs.join("bin/bash").is_file() {
        return Err(HostError::new("Android Linux rootfs must contain /bin/bash"));
    }
    Ok(rootfs)
}

/// Supplies the host-owned workspaces to both Android Linux launchers.
fn androidWorkspaceRoots() -> HostResult<Vec<std::path::PathBuf>> {
    ["OPERIT_ANDROID_INTERNAL_ROOT", "OPERIT_ANDROID_RUNTIME_ROOT", "OPERIT_ANDROID_WORKSPACE_ROOT"]
        .into_iter().map(requiredAndroidRuntimePath).collect()
}

/// Validates directory-only Android binds and reserves runtime paths from custom mounts.
fn androidProfileMounts(profile: &operit_host_api::TerminalProfile) -> HostResult<Vec<TerminalMount>> {
    let mounts = profileMounts(profile)?;
    if mounts.is_empty() {
        return Ok(mounts);
    }
    let mut reserved = androidWorkspaceRoots()?;
    reserved.extend(["/dev", "/proc", "/sys", "/storage", "/sdcard", "/data/local/tmp"].map(std::path::PathBuf::from));
    for mount in &mounts {
        let source = Path::new(&mount.source);
        let target = Path::new(&mount.target);
        if !source.is_absolute() || (profile.terminal == PROOT_TERMINAL
            && (mount.source.contains(':') || mount.target.contains([':', '!']))) {
            return Err(HostError::new("Invalid path syntax for Android terminal mount"));
        }
        if reserved.iter().any(|path| target.starts_with(path) || path.starts_with(target)) ||
            mounts.iter().any(|other| other.target != mount.target && target.starts_with(&other.target)) {
            return Err(HostError::new(format!("Overlapping terminal mount target: {}", mount.target)));
        }
        if profile.terminal == PROOT_TERMINAL && !source.is_dir() {
            return Err(HostError::new(format!("PRoot mount source is not an accessible directory: {}", mount.source)));
        }
    }
    Ok(mounts)
}

/// Enters a session-owned mount namespace before invoking chroot with literal arguments.
fn buildAndroidChrootCommand(rootfs: &Path, workingDir: &str, mounts: &[TerminalMount]) -> HostResult<AndroidPtyCommand> {
    let rootArguments = crate::terminal_privilege::requireRootArguments()?;
    let shell = "/system/bin/sh";
    let mut arguments = vec![
        "/system/bin/unshare".to_string(), "-m".into(), shell.into(), "-c".into(),
        include_str!("terminal_chroot.sh").into(), "operit-chroot".into(),
        rootfs.to_string_lossy().into_owned(), workingDir.into(), androidPtyPromptCommand(),
    ];
    for path in ["/proc", "/dev", "/sys", "/storage", "/sdcard", "/data/local/tmp"] {
        arguments.extend([path.into(), path.into(), "rbind".into()]);
    }
    let mut workspaceRoots = androidWorkspaceRoots()?;
    workspaceRoots.sort_unstable();
    workspaceRoots.dedup();
    for path in workspaceRoots {
        let path = path.to_string_lossy();
        arguments.extend([path.to_string(), path.to_string(), "bind".into()]);
    }
    for mount in mounts {
        arguments.extend([mount.source.clone(), mount.target.clone(), "bind".into()]);
    }
    let argv = if rootArguments.is_empty() {
        arguments.iter().map(|argument| cstring(argument)).collect::<HostResult<Vec<_>>>()?
    } else {
        let script = arguments.iter().map(|argument| quoteAndroidShellArgument(argument)).collect::<Vec<_>>().join(" ");
        [vec![shell.into(), "-c".into(), "exec \"$@\"".into(), "operit-root-chroot".into()],
            rootArguments, vec!["-c".into(), format!("exec {script}")]]
            .concat().iter().map(|argument| cstring(argument)).collect::<HostResult<Vec<_>>>()?
    };
    Ok(AndroidPtyCommand {
        executable: argv[0].clone(),
        argv,
        envp: ["PATH=/system/bin:/system/xbin", "TERM=xterm-256color"].into_iter().map(cstring).collect::<HostResult<Vec<_>>>()?,
        cwd: cstring("/")?,
        shizuku: false,
        dropRootToShell: false,
        workingDir: None,
        startupInput: None,
    })
}

/// Starts Bash using the profile's exact Linux launcher and custom mounts.
fn buildAndroidProfileCommand(
    profile: &operit_host_api::TerminalProfile,
    workingDir: &str,
) -> HostResult<AndroidPtyCommand> {
    let rootfs = androidProfileRootfs(profile)?;
    let mounts = androidProfileMounts(profile)?;
    if profile.terminal == CHROOT_TERMINAL {
        return buildAndroidChrootCommand(&rootfs, workingDir, &mounts);
    }
    let mut command = crate::runtime_common::buildAndroidProotCommandWithRootfs(
        "/bin/bash",
        Some(workingDir),
        &rootfs,
        &mounts,
    )?;
    command.env("TERM", "xterm-256color");
    command.env("PS1", "$PWD $ ");
    command.env("PROMPT_COMMAND", androidPtyPromptCommand());
    command.args(["--noprofile", "--norc", "-i"]);
    let executable = cstring(&command.get_program().to_string_lossy())?;
    let mut argv = vec![executable.clone()];
    for argument in command.get_args() {
        argv.push(cstring(&argument.to_string_lossy())?);
    }
    let mut environment: BTreeMap<std::ffi::OsString, std::ffi::OsString> =
        env::vars_os().collect();
    for (key, value) in command.get_envs() {
        match value {
            Some(value) => {
                environment.insert(key.into(), value.into());
            }
            None => {
                environment.remove(key);
            }
        }
    }
    let envp = environment
        .into_iter()
        .map(|(key, value)| {
            cstring(&format!(
                "{}={}",
                key.to_string_lossy(),
                value.to_string_lossy()
            ))
        })
        .collect::<HostResult<Vec<_>>>()?;
    Ok(AndroidPtyCommand {
        executable,
        argv,
        envp,
        cwd: cstringPath(
            command
                .get_current_dir()
                .ok_or_else(|| HostError::new("Proot process directory is required"))?,
        )?,
        shizuku: false,
        dropRootToShell: false,
        workingDir: None,
        startupInput: None,
    })
}

/// Builds the PTY startup command for an Android terminal type.
fn buildAndroidPtyCommand(terminalType: &str, workingDir: &str) -> HostResult<AndroidPtyCommand> {
    match terminalType {
        "bash" => buildAndroidBashPtyCommand(workingDir),
        "shell" => buildAndroidShellPtyCommand(workingDir),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for android host: {value}"
        ))),
    }
}

/// Builds the PTY startup command for Android proot Linux bash.
fn buildAndroidBashPtyCommand(workingDir: &str) -> HostResult<AndroidPtyCommand> {
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
    let promptCommand = androidPtyPromptCommand();
    let commandText = "source $HOME/common.sh && start_shell";
    let argv = vec![cstringPath(&bash)?, cstring("-c")?, cstring(commandText)?];
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
        cstring(&format!("TMPDIR={}", tmpDir.to_string_lossy()))?,
        cstring(&format!("PROOT_TMP_DIR={}", tmpDir.to_string_lossy()))?,
        cstring(&format!("OPERIT_WORKING_DIR={workDir}"))?,
        cstring("TERM=xterm-256color")?,
        cstring("LANG=en_US.UTF-8")?,
        cstring("PS1=$PWD $ ")?,
        cstring(&format!("PROMPT_COMMAND={promptCommand}"))?,
    ];
    androidLogInfo(&format!(
        "buildAndroidPtyCommand type=bash workingDir={workDir} runtimeDir={} internalRoot={} tmpDir={} bash={} loader={} nativeLibraryDir={}",
        runtimeDir.to_string_lossy(),
        internalRoot.to_string_lossy(),
        tmpDir.to_string_lossy(),
        bash.to_string_lossy(),
        loader.to_string_lossy(),
        nativeLibraryDir.to_string_lossy()
    ));
    Ok(AndroidPtyCommand {
        executable: cstringPath(&bash)?,
        argv,
        envp,
        cwd: cstringPath(&internalRoot)?,
        shizuku: false,
        dropRootToShell: false,
        workingDir: None,
        startupInput: None,
    })
}

/// Builds the PTY startup command for Android system shell.
fn buildAndroidShellPtyCommand(workingDir: &str) -> HostResult<AndroidPtyCommand> {
    let transport = crate::terminal_privilege::privileges()?.select()?;
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let nativeLibraryDir = requiredAndroidRuntimePath("OPERIT_ANDROID_NATIVE_LIBRARY_DIR")?;

    std::fs::create_dir_all(&tmpDir)?;

    let systemPath = env::var("PATH")
        .map_err(|error| HostError::new(format!("Android terminal PATH is required: {error}")))?;
    buildAndroidSystemShellCommand(
        transport,
        workingDir,
        &runtimeDir,
        &internalRoot,
        &tmpDir,
        &nativeLibraryDir,
        &systemPath,
    )
}

/// Maps app-private directories to shared storage for Shizuku system shells.
fn androidSystemShellWorkingDirectory(
    transport: &crate::terminal_privilege::ShellTransport,
    workingDir: &str,
    internalRoot: &Path,
) -> String {
    if *transport == crate::terminal_privilege::ShellTransport::Shizuku
        && Path::new(workingDir).starts_with(internalRoot)
    {
        "/sdcard".to_string()
    } else {
        workingDir.to_string()
    }
}

/// Builds the selected persistent shell with literal argv and transport-specific paths.
fn buildAndroidSystemShellCommand(
    transport: crate::terminal_privilege::ShellTransport,
    workingDir: &str,
    runtimeDir: &Path,
    internalRoot: &Path,
    tmpDir: &Path,
    nativeLibraryDir: &Path,
    systemPath: &str,
) -> HostResult<AndroidPtyCommand> {
    use crate::terminal_privilege::ShellTransport;

    let workDir = androidSystemShellWorkingDirectory(
        &transport,
        &nonBlank(workingDir, "working_directory")?,
        internalRoot,
    );
    let ldLibraryPath = format!(
        "{}:{}",
        nativeLibraryDir.to_string_lossy(),
        runtimeDir.to_string_lossy()
    );
    let commandText = "exec /system/bin/sh -i";
    let shell = Path::new("/system/bin/sh");
    let environment = if transport == ShellTransport::Shizuku {
        vec![
            "PATH=/system/bin:/system/xbin".to_string(),
            "HOME=/sdcard".to_string(),
            "TMPDIR=/data/local/tmp".to_string(),
            "TERM=xterm-256color".to_string(),
            "LANG=en_US.UTF-8".to_string(),
        ]
    } else {
        vec![
            format!("PATH={}:{}", runtimeDir.to_string_lossy(), systemPath),
            format!("HOME={}", internalRoot.to_string_lossy()),
            format!("LD_LIBRARY_PATH={ldLibraryPath}"),
            format!("TMPDIR={}", tmpDir.to_string_lossy()),
            "TERM=xterm-256color".to_string(),
            "LANG=en_US.UTF-8".to_string(),
        ]
    };
    let exports = environment
        .iter()
        .map(|value| format!("export {}", quoteAndroidShellArgument(value)))
        .collect::<Vec<_>>()
        .join("; ");
    let guard = if matches!(transport, ShellTransport::Root(_)) {
        "[ \"$(/system/bin/id -u)\" = 0 ] || exit 126; "
    } else {
        ""
    };
    let script = format!(
        "{guard}{exports}; cd {} || exit; {commandText}",
        quoteAndroidShellArgument(&workDir)
    );
    let argv = match &transport {
        ShellTransport::Root(arguments) => {
            let mut argv = vec![
                cstringPath(shell)?,
                cstring("-c")?,
                cstring("exec \"$@\"")?,
                cstring("operit-root-shell")?,
            ];
            for argument in arguments {
                argv.push(cstring(argument)?);
            }
            argv.extend([cstring("-c")?, cstring(&script)?]);
            argv
        }
        _ => vec![cstringPath(shell)?, cstring("-c")?, cstring(&script)?],
    };
    let envp = environment
        .iter()
        .map(|value| cstring(value))
        .collect::<HostResult<Vec<_>>>()?;
    androidLogInfo(&format!(
        "buildAndroidPtyCommand type=shell transport={transport:?} workingDir={workDir} runtimeDir={} internalRoot={} tmpDir={} nativeLibraryDir={}",
        runtimeDir.to_string_lossy(),
        internalRoot.to_string_lossy(),
        tmpDir.to_string_lossy(),
        nativeLibraryDir.to_string_lossy()
    ));
    Ok(AndroidPtyCommand {
        executable: cstringPath(shell)?,
        argv,
        envp,
        cwd: if transport == ShellTransport::Shizuku {
            cstring("/")?
        } else {
            cstringPath(&internalRoot)?
        },
        shizuku: transport == ShellTransport::Shizuku,
        dropRootToShell: false,
        workingDir: Some(workDir),
        startupInput: Some(include_str!("system_shell_init.sh").to_string()),
    })
}

/// Quotes literal environment values and paths without interpreting shell expressions.
fn quoteAndroidShellArgument(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn androidPtyPromptCommand() -> String {
    r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$(printf '%s' "$PWD" | base64 | tr -d '\n')" "$__operit_status""#.to_string()
}

fn androidTerminalDebugInfo(workingDir: &str) -> HostResult<BTreeMap<String, String>> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let rootfsDir = requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?;
    let runtimeRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_ROOT")?;
    let workspaceRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_WORKSPACE_ROOT")?;
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
    insertPathDebug(&mut info, "runtimeRoot", &runtimeRoot);
    insertPathDebug(&mut info, "workspaceRoot", &workspaceRoot);
    insertPathDebug(&mut info, "internalRoot", &internalRoot);
    insertPathDebug(&mut info, "tmpDir", &tmpDir);
    insertPathDebug(&mut info, "proot", &proot);
    insertPathDebug(&mut info, "bash", &bash);
    insertPathDebug(&mut info, "loader", &loader);
    insertPathDebug(&mut info, "busybox", &busybox);
    insertPathDebug(&mut info, "nativeLibraryDir", &nativeLibraryDir);
    insertPathDebug(&mut info, "rootfsBash", &rootfsBash);
    info.insert("workingDirectory".to_string(), workDir.clone());
    info.insert("promptCommand".to_string(), androidPtyPromptCommand());
    info.insert("bashTerminalType".to_string(), "bash".to_string());
    info.insert("shellTerminalType".to_string(), "shell".to_string());
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

/// Starts a local child or a remote Shizuku PTY exactly once.
fn startAndroidPtyProcess(
    command: &AndroidPtyCommand,
    rows: u16,
    cols: u16,
) -> HostResult<(AndroidPtyProcess, RawFd, RawFd)> {
    if command.shizuku {
        let request = serde_json::json!({
            "operation": "start", "argv": command.argv.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>(),
            "env": command.envp.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>(),
            "cwd": command.cwd.to_string_lossy(), "rows": rows, "cols": cols,
        });
        let response = crate::terminal_privilege::call(request)?;
        let pid = requiredPtyInteger(&response, "pid")?;
        let reader = requiredPtyInteger(&response, "readFd")?;
        let writer = requiredPtyInteger(&response, "writeFd")?;
        return Ok((AndroidPtyProcess::Shizuku(pid), reader, writer));
    }
    let (pid, fd) = forkPtyExecve(command, rows, cols)?;
    #[cfg(target_os = "android")]
    {
        let writer = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
        if writer < 0 {
            terminateAndroidPtyProcess(AndroidPtyProcess::Local(pid));
            unsafe {
                libc::close(fd);
            }
            return Err(HostError::new("Could not duplicate Android PTY writer"));
        }
        Ok((AndroidPtyProcess::Local(pid), fd, writer))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (pid, fd);
        unreachable!("forkPtyExecve rejects non-Android hosts")
    }
}

/// Rejects malformed process handles before they reach POSIX or Binder operations.
fn requiredPtyInteger(value: &serde_json::Value, key: &str) -> HostResult<i32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .and_then(|v| i32::try_from(v).ok())
        .filter(|v| *v >= 0)
        .ok_or_else(|| HostError::new(format!("Invalid Android PTY {key}")))
}

/// Stops a session through the same security context that created it.
fn terminateAndroidPtyProcess(process: AndroidPtyProcess) {
    match process {
        AndroidPtyProcess::Shizuku(pid) => {
            if let Err(error) = crate::terminal_privilege::call(
                serde_json::json!({"operation": "close", "pid": pid}),
            ) {
                androidLogError(&format!("Shizuku terminal close failed: {error}"));
            }
        }
        AndroidPtyProcess::Local(pid) => {
            #[cfg(target_os = "android")]
            unsafe {
                let mut status = 0;
                // A reaped PID can already belong to an unrelated process.
                if libc::waitpid(pid, &mut status, libc::WNOHANG) != 0 {
                    return;
                }
                libc::kill(pid, libc::SIGHUP);
                libc::kill(pid, libc::SIGKILL);
                while libc::waitpid(pid, &mut status, 0) < 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR)
                {
                }
            }
            #[cfg(not(target_os = "android"))]
            let _ = pid;
        }
    }
}

/// Resizes the remote controlling PTY rather than assuming a local child.
fn resizeAndroidPtyProcess(
    process: AndroidPtyProcess,
    fd: RawFd,
    rows: u16,
    cols: u16,
) -> HostResult<()> {
    match process {
        AndroidPtyProcess::Local(_) => setPtyWindowSize(fd, rows, cols),
        AndroidPtyProcess::Shizuku(pid) => crate::terminal_privilege::call(
            serde_json::json!({"operation": "resize", "pid": pid, "rows": rows, "cols": cols}),
        )
        .map(|_| ()),
    }
}

/// A root-started Shizuku (e.g. KernelSU) leaks uid 0 into its UserService
/// children; the terminal promises shell identity, so drop before exec or die.
/// Mirrors Operit1's shell_identity_launcher: groups → gid → uid → setcon.
#[cfg(target_os = "android")]
unsafe fn dropRootToShellIdentity() {
    const SHELL_UID: libc::uid_t = 2000;
    const SHELL_GID: libc::gid_t = 2000;
    const SHELL_GROUPS: [libc::gid_t; 12] = [
        2000, // shell
        1004, // input
        1007, // log
        1011, // adb
        1015, // sdcard_rw
        1028, // sdcard_r
        3001, // net_bt_admin
        3002, // net_bt
        3003, // inet
        3006, // net_bw_stats
        3009, // readproc
        3011, // uhid
    ];
    let fail = |message: &[u8]| -> ! {
        libc::write(libc::STDERR_FILENO, message.as_ptr().cast(), message.len());
        libc::_exit(126);
    };
    if libc::setgroups(SHELL_GROUPS.len(), SHELL_GROUPS.as_ptr()) != 0 {
        fail(b"setgroups(shell) failed\n");
    }
    if libc::setgid(SHELL_GID) != 0 {
        fail(b"setgid(2000) failed\n");
    }
    if libc::setuid(SHELL_UID) != 0 {
        fail(b"setuid(2000) failed\n");
    }
    // Best effort: some su domains cannot dyntransition to u:r:shell:s0, and
    // uid 2000 already removes root DAC powers. Drop uid first because the
    // shell domain would deny setuid/setgid.
    let context = b"u:r:shell:s0\0";
    let fd = libc::open(
        b"/proc/self/attr/current\0".as_ptr().cast(),
        libc::O_WRONLY | libc::O_CLOEXEC,
    );
    let switched = fd >= 0 && {
        let written = libc::write(fd, context.as_ptr().cast(), context.len());
        libc::close(fd);
        written == context.len() as libc::ssize_t
    };
    if !switched {
        let message = b"setcon(u:r:shell:s0) failed; continuing as uid 2000\n";
        libc::write(libc::STDERR_FILENO, message.as_ptr().cast(), message.len());
    }
    if libc::getuid() != SHELL_UID || libc::getgid() != SHELL_GID {
        fail(b"shell identity verification failed\n");
    }
}

/// Re-execs a standalone Android Host after applying the terminal's shell identity.
#[cfg(target_os = "android")]
pub fn androidExecShellProcess(arguments: &[String]) -> HostResult<()> {
    let arguments = arguments
        .iter()
        .map(|argument| cstring(argument))
        .collect::<HostResult<Vec<_>>>()?;
    let executable = arguments
        .first()
        .ok_or_else(|| HostError::new("Missing Android shell executable"))?;
    let mut pointers = arguments
        .iter()
        .map(|argument| argument.as_ptr())
        .collect::<Vec<_>>();
    pointers.push(std::ptr::null());
    unsafe {
        if libc::geteuid() == 0 {
            dropRootToShellIdentity();
        }
        if libc::getuid() != 2000 || libc::getgid() != 2000 {
            return Err(HostError::new("Android UI server requires shell uid/gid 2000"));
        }
        libc::execv(executable.as_ptr(), pointers.as_ptr());
    }
    Err(HostError::new(format!(
        "Android shell exec failed: {}",
        std::io::Error::last_os_error()
    )))
}

/// Forks the PTY child in the current process's Android security context.
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
                if command.dropRootToShell && libc::geteuid() == 0 {
                    dropRootToShellIdentity();
                }
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
        if unsafe { libc::fcntl(masterFd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            terminateAndroidPtyProcess(AndroidPtyProcess::Local(pid));
            unsafe {
                libc::close(masterFd);
            }
            return Err(HostError::new("Could not mark Android PTY close-on-exec"));
        }
        Ok((pid, masterFd))
    }
}

/// Forks a noninteractive child with separate stdout/stderr pipes so binary
/// output (e.g. screencap PNG) never passes through PTY line discipline.
#[cfg(target_os = "android")]
fn forkPipeExecve(command: &AndroidPtyCommand) -> HostResult<(AndroidPid, RawFd, RawFd)> {
    let mut stdoutPipe: [libc::c_int; 2] = [-1, -1];
    let mut stderrPipe: [libc::c_int; 2] = [-1, -1];
    unsafe {
        if libc::pipe(stdoutPipe.as_mut_ptr()) != 0 {
            return Err(HostError::new("stdout pipe failed"));
        }
        if libc::pipe(stderrPipe.as_mut_ptr()) != 0 {
            libc::close(stdoutPipe[0]);
            libc::close(stdoutPipe[1]);
            return Err(HostError::new("stderr pipe failed"));
        }
        let pid = libc::fork();
        if pid < 0 {
            for fd in stdoutPipe.iter().chain(stderrPipe.iter()) {
                libc::close(*fd);
            }
            return Err(HostError::new("fork failed"));
        }
        if pid == 0 {
            if libc::dup2(stdoutPipe[1], libc::STDOUT_FILENO) < 0
                || libc::dup2(stderrPipe[1], libc::STDERR_FILENO) < 0
            {
                libc::write(
                    stderrPipe[1],
                    b"dup2 failed\n".as_ptr().cast(),
                    b"dup2 failed\n".len(),
                );
                libc::_exit(1);
            }
            for fd in stdoutPipe.iter().chain(stderrPipe.iter()) {
                libc::close(*fd);
            }
            let fail = |message: &[u8]| -> ! {
                libc::write(libc::STDERR_FILENO, message.as_ptr().cast(), message.len());
                libc::_exit(1);
            };
            let null = libc::open(b"/dev/null\0".as_ptr().cast(), libc::O_RDONLY);
            if null < 0 || libc::dup2(null, libc::STDIN_FILENO) < 0 {
                fail(b"stdin /dev/null failed\n");
            }
            if null > libc::STDERR_FILENO {
                libc::close(null);
            }
            // After dup2 so a refused identity drop is visible in collected stderr.
            if command.dropRootToShell && libc::geteuid() == 0 {
                dropRootToShellIdentity();
            }
            if libc::chdir(command.cwd.as_ptr()) != 0 {
                fail(b"chdir failed\n");
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
            fail(b"execve failed\n");
        }
        libc::close(stdoutPipe[1]);
        libc::close(stderrPipe[1]);
        for fd in [stdoutPipe[0], stderrPipe[0]] {
            if libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) < 0 {
                terminateAndroidPtyProcess(AndroidPtyProcess::Local(pid));
                libc::close(stdoutPipe[0]);
                libc::close(stderrPipe[0]);
                return Err(HostError::new("Could not mark Android exec pipes close-on-exec"));
            }
        }
        Ok((pid, stdoutPipe[0], stderrPipe[0]))
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


/// Completes partial writes and retries interrupts so PTY input is never truncated.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn writePtyBytesWith(
    mut write: impl FnMut(&[u8]) -> std::io::Result<usize>,
    data: &[u8],
) -> std::io::Result<usize> {
    let mut written = 0;
    while written < data.len() {
        match write(&data[written..]) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "PTY write accepted no bytes",
                ));
            }
            Ok(count) => written += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(written)
}

/// Writes complete PTY input with bounded waits for nonblocking backpressure.
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
        let deadline = Instant::now() + Duration::from_secs(10);
        writePtyBytesWith(
            |remaining| loop {
                let count =
                    unsafe { libc::write(fd, remaining.as_ptr().cast(), remaining.len()) };
                if count >= 0 {
                    return Ok(count as usize);
                }
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(error);
                }
                let remainingTime = deadline.saturating_duration_since(Instant::now());
                if remainingTime.is_zero() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Android PTY input submission timed out",
                    ));
                }
                let mut descriptor = libc::pollfd { fd, events: libc::POLLOUT, revents: 0 };
                let ready = unsafe {
                    libc::poll(
                        &mut descriptor,
                        1,
                        remainingTime.as_millis().min(i32::MAX as u128) as i32,
                    )
                };
                if ready < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            },
            data,
        )
        .map_err(|error| {
            androidLogError(&format!("write failed fd={fd} bytes={}: {error}", data.len()));
            HostError::new(format!("Android PTY input failed: {error}"))
        })
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

/// Polls locally owned children or the Shizuku-owned remote process.
fn pollPidExitCode(process: AndroidPtyProcess) -> HostResult<Option<i32>> {
    let pid = match process {
        AndroidPtyProcess::Shizuku(pid) => {
            let response = crate::terminal_privilege::call(
                serde_json::json!({"operation": "poll", "pid": pid}),
            )?;
            return match response.get("exitCode") {
                Some(serde_json::Value::Null) => Ok(None),
                Some(value) => value
                    .as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .map(Some)
                    .ok_or_else(|| HostError::new("Invalid Shizuku terminal exit code")),
                None => Err(HostError::new("Missing Shizuku terminal exit code")),
            };
        }
        AndroidPtyProcess::Local(pid) => pid,
    };
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

/// Parses one privileged launch request; every child must keep the shell identity.
#[cfg(target_os = "android")]
fn privilegedPtyCommand(request: &serde_json::Value) -> HostResult<AndroidPtyCommand> {
    use serde_json::Value;
    let strings = |key: &str| -> HostResult<Vec<CString>> {
        request
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new(format!("Missing PTY {key}")))?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| HostError::new("Invalid PTY argument"))
                    .and_then(cstring)
            })
            .collect()
    };
    let argv = strings("argv")?;
    let executable = argv
        .first()
        .ok_or_else(|| HostError::new("Empty PTY argv"))?
        .clone();
    let cwd = request
        .get("cwd")
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::new("Missing PTY cwd"))?;
    Ok(AndroidPtyCommand {
        executable,
        argv,
        envp: strings("env")?,
        cwd: cstring(cwd)?,
        shizuku: false,
        dropRootToShell: true,
        workingDir: None,
        startupInput: None,
    })
}

/// Provides PTY primitives to the privileged Binder service without invoking the app runtime.
#[cfg(target_os = "android")]
pub fn androidPrivilegedPtyOperation(request: &str) -> HostResult<String> {
    use serde_json::{json, Value};
    let request: Value =
        serde_json::from_str(request).map_err(|e| HostError::new(e.to_string()))?;
    if request.get("operation").and_then(Value::as_str) != Some("start")
        && request.get("operation").and_then(Value::as_str) != Some("exec")
        && requiredPtyInteger(&request, "pid")? == 0
    {
        return Err(HostError::new("Android PTY PID must be positive"));
    }
    let response = match request.get("operation").and_then(Value::as_str) {
        Some("start") => {
            let command = privilegedPtyCommand(&request)?;
            let rows = u16::try_from(requiredPtyInteger(&request, "rows")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            let cols = u16::try_from(requiredPtyInteger(&request, "cols")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            let (pid, fd) = forkPtyExecve(&command, rows, cols)?;
            json!({"pid": pid, "fd": fd})
        }
        Some("exec") => {
            let command = privilegedPtyCommand(&request)?;
            let (pid, stdoutFd, stderrFd) = forkPipeExecve(&command)?;
            json!({"pid": pid, "stdoutFd": stdoutFd, "stderrFd": stderrFd})
        }
        Some("poll") => {
            json!({"exitCode": pollPidExitCode(AndroidPtyProcess::Local(requiredPtyInteger(&request, "pid")?))?})
        }
        Some("close") => {
            terminateAndroidPtyProcess(AndroidPtyProcess::Local(requiredPtyInteger(
                &request, "pid",
            )?));
            json!({})
        }
        Some("resize") => {
            let rows = u16::try_from(requiredPtyInteger(&request, "rows")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            let cols = u16::try_from(requiredPtyInteger(&request, "cols")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            setPtyWindowSize(requiredPtyInteger(&request, "fd")?, rows, cols)?;
            json!({})
        }
        _ => return Err(HostError::new("Unsupported privileged PTY operation")),
    };
    Ok(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document_resource() -> String {
        operit_host_api::FileSystemResource::FileSystemResource {
            backend: "android_documents".into(),
            root: "content://com.android.externalstorage.documents/tree/primary%3AProjects".into(),
            path: "项目 with spaces".into(),
        }.encode().unwrap()
    }

    #[test]
    fn document_working_directory_is_resolved_by_android_provider() {
        let resource = document_resource();
        let directory = resolveAndroidNativeWorkingDirectory(&resource, &|request| {
            assert_eq!(request["operation"], "resolveNativeDirectory");
            assert_eq!(request["path"], resource);
            Ok(serde_json::json!("/storage/emulated/0/Projects/项目 with spaces"))
        }).unwrap();
        assert_eq!(directory, "/storage/emulated/0/Projects/项目 with spaces");
        for (terminal, kind) in [("proot", "bash"), ("android-system", "shell")] {
            let actual = AndroidTerminalHost::default().resolveWorkingDirectory(
                terminal, kind, "/virtual/android/documents/id/项目 with spaces", &|_| Ok(directory.clone()),
            ).unwrap();
            assert_eq!(actual, directory);
        }
    }

    #[test]
    fn native_and_guest_directories_do_not_call_document_provider() {
        let reject = |_: serde_json::Value| panic!("provider must not receive native paths");
        assert_eq!(resolveAndroidNativeWorkingDirectory("/storage/emulated/0/work", &reject).unwrap(),
            "/storage/emulated/0/work");
        let host = AndroidTerminalHost::default();
        assert_eq!(host.resolveWorkingDirectory("proot", "bash", "/virtual/linux/root", &|_| {
            panic!("guest paths must not reach host resolver")
        }).unwrap(), "/root");
    }

    #[test]
    fn document_directory_errors_and_invalid_provider_responses_are_rejected() {
        let resource = document_resource();
        let error = resolveAndroidNativeWorkingDirectory(&resource, &|_| {
            Err(HostError::new("Document tree permission was revoked"))
        }).unwrap_err();
        assert!(error.message.contains("permission was revoked"));
        for value in [serde_json::Value::Null, serde_json::json!("relative/path"),
            serde_json::json!("/storage/bad\u{0}path")] {
            assert!(resolveAndroidNativeWorkingDirectory(&resource, &|_| Ok(value.clone())).is_err());
        }
        let unsupported = resource.replace("android_documents", "other_backend");
        assert!(resolveAndroidNativeWorkingDirectory(&unsupported, &|_| panic!("unsupported backend")).is_err());
    }

    /// Builds production shell argv without requiring an Android runtime or device.
    fn system_shell(
        transport: crate::terminal_privilege::ShellTransport,
        directory: &str,
    ) -> AndroidPtyCommand {
        buildAndroidSystemShellCommand(
            transport,
            directory,
            Path::new("/private/runtime"),
            Path::new("/private/app"),
            Path::new("/private/tmp"),
            Path::new("/private/lib"),
            "/system/bin",
        )
        .unwrap()
    }

    /// A Root session launches su once and verifies its identity before accepting commands.
    #[test]
    fn root_system_shell_uses_verified_su_and_persistent_prompt() {
        let command = system_shell(
            crate::terminal_privilege::ShellTransport::Root(vec![
                "/custom path/su".into(),
                "--flag".into(),
            ]),
            "/private/app",
        );
        let argv = command
            .argv
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            &argv[..7],
            &[
                "/system/bin/sh",
                "-c",
                "exec \"$@\"",
                "operit-root-shell",
                "/custom path/su",
                "--flag",
                "-c"
            ]
        );
        assert!(argv[7].starts_with("[ \"$(/system/bin/id -u)\" = 0 ] || exit 126;"));
        assert!(argv[7].ends_with("exec /system/bin/sh -i"));
        assert!(command.startupInput.as_ref().unwrap().contains("OperitPrompt="));
        assert!(!command.shizuku);
    }

    /// Shizuku must not inherit private app paths that the ADB shell UID cannot access.
    #[test]
    fn shizuku_system_shell_has_accessible_environment_and_default_directory() {
        for directory in ["/private/app", "/private/app/workspaces/project"] {
            let command = system_shell(
                crate::terminal_privilege::ShellTransport::Shizuku,
                directory,
            );
            assert!(command.shizuku);
            assert_eq!(command.cwd.to_str().unwrap(), "/");
            assert_eq!(command.workingDir.as_deref(), Some("/sdcard"));
            assert!(command.argv[2]
                .to_str()
                .unwrap()
                .contains("cd '/sdcard' || exit;"));
            for value in command.envp {
                assert!(!value.to_str().unwrap().contains("/private/"));
            }
        }
        for directory in ["/sdcard/custom", "/private/app-other/workspaces/project"] {
            let explicit = system_shell(
                crate::terminal_privilege::ShellTransport::Shizuku,
                directory,
            );
            assert_eq!(explicit.workingDir.as_deref(), Some(directory));
        }
    }

    /// Literal paths cannot inject commands and the app shell keeps its original private directory.
    #[test]
    fn app_system_shell_preserves_directory_and_quotes_literals() {
        let directory = "/private/app/a'b; touch bad";
        let command = system_shell(crate::terminal_privilege::ShellTransport::App, directory);
        assert!(!command.shizuku);
        assert_eq!(command.workingDir.as_deref(), Some(directory));
        assert!(command.argv[2]
            .to_str()
            .unwrap()
            .contains("cd '/private/app/a'\\''b; touch bad' || exit;"));
        assert_eq!(command.argv[0].to_str().unwrap(), "/system/bin/sh");
        assert_eq!(quoteAndroidShellArgument("$(id)"), "'$(id)'");
    }

    /// Resolves proot guest paths without invoking the Android host VFS mapper.
    #[test]
    fn manual_proot_directory_uses_guest_namespace() {
        let host = AndroidTerminalHost::new();
        let resolveHost = |_: &str| Err(HostError::new("/virtual/linux is not mounted"));
        assert_eq!(
            host.resolveWorkingDirectory("proot", "bash", "/virtual/linux/root", &resolveHost)
                .unwrap(),
            "/root",
        );
    }

    /// Leaves Android system-shell mount errors intact instead of selecting proot.
    #[test]
    fn manual_system_shell_preserves_host_mount_failure() {
        let host = AndroidTerminalHost::new();
        let resolveHost = |_: &str| Err(HostError::new("/virtual/linux is not mounted"));
        assert_eq!(
            host.resolveWorkingDirectory(
                "android-system",
                "shell",
                "/virtual/linux/root",
                &resolveHost
            )
            .unwrap_err()
            .message,
            "/virtual/linux is not mounted",
        );
    }

    /// Retains the actual proot startup failure in the returned terminal error.
    #[test]
    fn startup_errors_include_process_output() {
        let error = androidPtyStartupError(b"proot: execve(/bin/bash): Permission denied\r\n");
        assert_eq!(
            error.message,
            "Timed out waiting for Android terminal prompt\nStartup output:\nproot: execve(/bin/bash): Permission denied",
        );
    }

    /// Ensures the native system shell and proot Bash have distinct implementation identities.
    #[test]
    fn interpreter_types_select_exact_android_implementations() {
        assert_eq!(
            androidTerminalName("shell").unwrap(),
            ANDROID_SYSTEM_TERMINAL
        );
        assert_eq!(androidTerminalName("bash").unwrap(), PROOT_TERMINAL);
        assert_ne!(sessionKey("shell", "chat"), sessionKey("bash", "chat"));
    }

    /// Rejects an unsupported explicit interpreter before creating any PTY or accessing runtime paths.
    #[test]
    fn typed_session_rejects_unsupported_interpreter() {
        let host = AndroidTerminalHost::new();
        let error = host
            .createOrGetTypedSession("chat", "powershell")
            .unwrap_err();
        assert_eq!(
            error.message,
            "Unsupported Android terminal type: powershell"
        );
        assert!(host.lockState().unwrap().ptySessions.is_empty());
    }

    /// Rejects an empty explicit interpreter instead of creating the default proot session.
    #[test]
    fn typed_session_rejects_empty_interpreter() {
        let host = AndroidTerminalHost::new();
        let error = host.createOrGetTypedSession("chat", " ").unwrap_err();
        assert_eq!(error.message, "type parameter is required");
        assert!(host.lockState().unwrap().ptySessions.is_empty());
    }

    /// A short write must not truncate the command: the remainder is submitted in order.
    #[test]
    fn pty_write_completes_partial_writes() {
        let data = b"printf '%s\\n' '327429f1'\r";
        let mut submitted = Vec::new();
        let written = writePtyBytesWith(
            |remaining| {
                let count = remaining.len().min(3);
                submitted.extend_from_slice(&remaining[..count]);
                Ok(count)
            },
            data,
        )
        .unwrap();
        assert_eq!(written, data.len());
        assert_eq!(submitted, data);
    }

    /// An interrupted write is retried instead of failing or dropping the command.
    #[test]
    fn pty_write_retries_interrupted_writes() {
        let data = b"echo '5c8a733d'\r";
        let mut calls = 0;
        let mut submitted = Vec::new();
        let written = writePtyBytesWith(
            |remaining| {
                calls += 1;
                if calls <= 2 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "EINTR",
                    ));
                }
                let count = remaining.len().min(4);
                submitted.extend_from_slice(&remaining[..count]);
                Ok(count)
            },
            data,
        )
        .unwrap();
        assert_eq!(written, data.len());
        assert_eq!(submitted, data);
    }

    /// Zero progress and hard failures surface as errors; empty input never writes.
    #[test]
    fn pty_write_rejects_zero_progress_and_hard_errors() {
        let zero = writePtyBytesWith(|_| Ok(0), b"x").unwrap_err();
        assert_eq!(zero.kind(), std::io::ErrorKind::WriteZero);
        let broken = writePtyBytesWith(
            |_| Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "EPIPE")),
            b"x",
        )
        .unwrap_err();
        assert_eq!(broken.kind(), std::io::ErrorKind::BrokenPipe);
        assert_eq!(writePtyBytesWith(|_| panic!("empty input must not write"), b"").unwrap(), 0);
    }
}
