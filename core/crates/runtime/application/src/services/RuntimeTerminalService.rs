use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::TerminalProfiles::{
    TerminalDistribution, TerminalInstallProgressCallback, TerminalProfileSettings,
    TerminalProfileSupport,
};
use operit_host_api::{HostRuntimeTaskSchedulerHost, TerminalHost, TerminalSessionListEntry};
use operit_store::{
    PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow},
    RuntimeStorePaths::RuntimeStorePaths,
};
use serde::{Deserialize, Serialize};

use operit_host_api::HostManager::HostManager;
use operit_util::stream::HotStream::MutableSharedStreamImpl;
use operit_util::stream::Stream::{CollectFuture, Stream};
use operit_vfs::VfsPath;
use operit_vfs::VisualFileSystem::VisualFileSystem;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Published terminal session metadata for UI session lists.
pub struct RuntimeTerminalSessionInfo {
    pub sessionId: String,
    pub sessionName: String,
    pub platform: String,
    pub terminal: String,
    pub terminalType: String,
    pub sessionKind: String,
    pub workingDir: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Current terminal screen snapshot returned by the host.
pub struct RuntimeTerminalScreen {
    pub sessionId: String,
    pub platform: String,
    pub terminal: String,
    pub terminalType: String,
    pub rows: i32,
    pub cols: i32,
    pub content: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Describes one terminal type that may be created by the active host.
pub struct RuntimeTerminalTypeInfo {
    pub terminal: String,
    pub terminalType: String,
    pub available: bool,
    pub description: String,
    #[serde(default)]
    pub nameKey: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Describes all terminal types exposed by the active host.
pub struct RuntimeTerminalInfo {
    pub platform: String,
    pub terminal: String,
    pub terminalType: String,
    pub types: Vec<RuntimeTerminalTypeInfo>,
}

/// A bounded installation job that can be observed after reconnecting to the Core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTerminalInstall {
    pub installId: String,
    pub distributionId: String,
    pub name: String,
    pub state: String,
    pub downloadedBytes: u64,
    pub totalBytes: u64,
    pub profileId: Option<String>,
    pub error: Option<String>,
}

struct TerminalInstallJob {
    host: std::sync::Weak<dyn TerminalHost>,
    control: operit_host_api::HttpDownloadControl,
    snapshot: RuntimeTerminalInstall,
}

static TERMINAL_INSTALL_JOBS: OnceLock<Mutex<Vec<TerminalInstallJob>>> = OnceLock::new();

/// Returns the bounded host-owned installation registry shared by service handles.
fn terminal_install_jobs() -> &'static Mutex<Vec<TerminalInstallJob>> {
    TERMINAL_INSTALL_JOBS.get_or_init(|| Mutex::new(Vec::new()))
}

impl TerminalInstallJob {
    /// Reports whether this attempt has released its installation resources.
    fn isFinished(&self) -> bool {
        matches!(self.snapshot.state.as_str(), "completed" | "failed" | "cancelled")
    }
}

/// Facade over host terminal APIs and shared terminal output streams.
pub struct RuntimeTerminalService {
    terminalHost: Arc<dyn TerminalHost>,
    context: HostManager,
}

#[derive(Clone, Debug)]
/// Stream wrapper exposing PTY output to Kotlin-style collectors.
pub struct RuntimeTerminalPtyOutputStream {
    upstream: MutableSharedStreamImpl<String>,
}

impl RuntimeTerminalPtyOutputStream {
    /// Wraps a shared PTY output stream.
    pub fn new(upstream: MutableSharedStreamImpl<String>) -> Self {
        Self { upstream }
    }
}

impl Stream for RuntimeTerminalPtyOutputStream {
    type Item = String;

    /// Collects PTY output asynchronously until the host closes the stream.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        self.upstream.collect(collector)
    }
}

#[derive(Clone)]
struct TerminalPtyOutputEntry {
    stream: MutableSharedStreamImpl<String>,
}

static TERMINAL_PTY_OUTPUT_STREAMS: OnceLock<Mutex<HashMap<String, TerminalPtyOutputEntry>>> =
    OnceLock::new();
static TERMINAL_SESSIONS_FLOW: OnceLock<MutableStateFlow<Vec<RuntimeTerminalSessionInfo>>> =
    OnceLock::new();

fn terminal_pty_output_streams() -> &'static Mutex<HashMap<String, TerminalPtyOutputEntry>> {
    TERMINAL_PTY_OUTPUT_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn terminal_sessions_flow() -> &'static MutableStateFlow<Vec<RuntimeTerminalSessionInfo>> {
    TERMINAL_SESSIONS_FLOW.get_or_init(|| mutableStateFlow(Vec::new()))
}

fn runtime_terminal_session_info(session: TerminalSessionListEntry) -> RuntimeTerminalSessionInfo {
    RuntimeTerminalSessionInfo {
        sessionId: session.sessionId,
        sessionName: session.sessionName,
        platform: session.platform,
        terminal: session.terminal,
        terminalType: session.terminalType,
        sessionKind: session.sessionKind,
        workingDir: session.workingDir,
        commandRunning: session.commandRunning,
    }
}

fn load_terminal_sessions(
    terminalHost: &dyn TerminalHost,
) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
    terminalHost
        .listSessions()
        .map_err(|error| error.message)
        .map(|sessions| {
            sessions
                .into_iter()
                .map(runtime_terminal_session_info)
                .collect()
        })
}

fn publish_terminal_sessions(
    terminalHost: &dyn TerminalHost,
) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
    let sessions = load_terminal_sessions(terminalHost)?;
    terminal_sessions_flow().set_value(sessions.clone());
    Ok(sessions)
}

/// Publishes the current terminal session snapshot for non-service terminal callers.
pub fn publish_terminal_sessions_for_host(
    terminalHost: &dyn TerminalHost,
) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
    publish_terminal_sessions(terminalHost)
}

fn close_terminal_pty_output_stream(sessionId: &str) {
    let entry = terminal_pty_output_streams()
        .lock()
        .expect("terminal pty output streams mutex poisoned")
        .remove(sessionId);
    if let Some(entry) = entry {
        entry.stream.close();
    }
}

fn start_terminal_pty_output_reader(
    terminalHost: Arc<dyn TerminalHost>,
    taskScheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    sessionId: String,
    stream: MutableSharedStreamImpl<String>,
) {
    let scheduledTaskScheduler = taskScheduler.clone();
    taskScheduler
        .scheduleHostRuntimeAsyncTask(
            "operit-terminal-pty-output",
            Box::new(move || {
                Box::pin(async move {
                    // Keep one request thread and yield to the shared timer instead
                    // of creating a new OS thread for every 40 ms polling turn.
                    while poll_terminal_pty_output(terminalHost.as_ref(), &sessionId, &stream) {
                        if scheduledTaskScheduler
                            .waitForHostRuntimeDelay(40)
                            .await
                            .is_err()
                        {
                            close_terminal_pty_output_stream(&sessionId);
                            break;
                        }
                    }
                })
            }),
        )
        .expect("terminal PTY output task must be scheduled");
}

/// Reads one batch; false terminates the session's single asynchronous reader.
fn poll_terminal_pty_output(
    terminalHost: &dyn TerminalHost,
    sessionId: &str,
    stream: &MutableSharedStreamImpl<String>,
) -> bool {
    if !terminal_pty_output_streams()
        .lock()
        .expect("terminal pty output streams mutex poisoned")
        .contains_key(sessionId)
    {
        return false;
    }
    match terminalHost.readPtySession(sessionId) {
        Ok(data) if !data.is_empty() => stream.emit(STANDARD.encode(data)),
        Ok(_) => {}
        Err(_) => {
            close_terminal_pty_output_stream(sessionId);
            return false;
        }
    }

    match terminalHost.pollPtyExitCode(sessionId) {
        Ok(Some(_)) => {
            // Drain bytes arriving between the read and the exit observation.
            if let Ok(data) = terminalHost.readPtySession(sessionId) {
                if !data.is_empty() {
                    stream.emit(STANDARD.encode(data));
                }
            }
            publish_terminal_sessions(terminalHost)
                .expect("TerminalHost.listSessions must succeed after PTY exit");
            close_terminal_pty_output_stream(sessionId);
            false
        }
        Ok(None) => true,
        Err(_) => {
            close_terminal_pty_output_stream(sessionId);
            false
        }
    }
}

impl RuntimeTerminalService {
    #[allow(non_snake_case)]
    /// Creates a terminal service from application host context.
    pub fn getInstance(context: &HostManager) -> Self {
        Self {
            terminalHost: context
                .terminalHost
                .clone()
                .expect("TerminalHost must be configured for RuntimeTerminalService"),
            context: context.clone(),
        }
    }

    #[allow(non_snake_case)]
    /// Lists terminal sessions currently known by the host.
    pub fn listTerminalSessions(&self) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
        publish_terminal_sessions(self.terminalHost.as_ref())
    }

    #[allow(non_snake_case)]
    /// Returns a state flow of published terminal sessions.
    pub fn terminalSessionsFlow(
        &self,
    ) -> Result<StateFlow<Vec<RuntimeTerminalSessionInfo>>, String> {
        publish_terminal_sessions(self.terminalHost.as_ref())?;
        Ok(terminal_sessions_flow().asStateFlow())
    }

    #[allow(non_snake_case)]
    /// Returns the host-declared terminal type for manual PTY creation.
    pub fn defaultTerminalType(&self) -> Result<String, String> {
        self.terminalHost
            .terminalInfo()
            .map(|info| info.terminalType)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Returns every terminal type that the active host exposes to users.
    pub fn terminalInfo(&self) -> Result<RuntimeTerminalInfo, String> {
        let info = self
            .terminalHost
            .terminalInfo()
            .map_err(|error| error.message)?;
        let settings = self
            .terminalHost
            .terminalProfileSettings()
            .map_err(|error| error.message)?;
        let selected = settings
            .profiles
            .iter()
            .find(|profile| profile.id == settings.defaultProfileId)
            .ok_or_else(|| "Default terminal profile is missing".to_string())?;
        Ok(RuntimeTerminalInfo {
            platform: info.platform,
            terminal: format!("profile:{}", selected.id),
            terminalType: selected.terminalType.clone(),
            types: settings
                .profiles
                .into_iter()
                .map(|profile| {
                    let available = !profile.options.is_empty()
                        || info.types.iter().any(|entry| {
                            entry.terminal == profile.terminal
                                && entry.terminalType == profile.terminalType
                                && entry.available
                        });
                    RuntimeTerminalTypeInfo {
                        terminal: format!("profile:{}", profile.id),
                        terminalType: profile.terminalType,
                        available,
                        description: profile.name,
                        nameKey: profile.nameKey,
                    }
                })
                .collect(),
        })
    }

    /// Reads terminal profiles owned by the active Core device.
    pub fn terminalProfileSettings(&self) -> Result<TerminalProfileSettings, String> {
        self.terminalHost
            .terminalProfileSettings()
            .map_err(|error| error.message)
    }

    /// Lists editable fields registered by each platform terminal implementation.
    pub fn terminalProfileSupport(&self) -> Result<Vec<TerminalProfileSupport>, String> {
        self.terminalHost
            .terminalProfileSupport()
            .map_err(|error| error.message)
    }

    /// Persists an explicitly selected default and custom launch configurations for new sessions.
    pub fn saveTerminalProfileSettings(
        &self,
        settings: TerminalProfileSettings,
    ) -> Result<TerminalProfileSettings, String> {
        self.terminalHost
            .saveTerminalProfileSettings(settings)
            .map_err(|error| error.message)?;
        self.terminalHost
            .terminalProfileSettings()
            .map_err(|error| error.message)
    }

    /// Lists downloadable distributions supported by the connected Core host.
    pub fn terminalDistributions(&self) -> Result<Vec<TerminalDistribution>, String> {
        self.terminalHost
            .terminalDistributions()
            .map_err(|error| error.message)
    }

    /// Starts a cancellable installation on the host pool and immediately returns its progress handle.
    pub fn startTerminalDistributionInstall(
        &self,
        distributionId: String,
        name: String,
    ) -> Result<RuntimeTerminalInstall, String> {
        if name.trim().is_empty() || name.contains('\0') {
            return Err("Terminal name must not be blank or contain NUL".into());
        }
        let distribution = self
            .terminalHost
            .terminalDistributions()
            .map_err(|error| error.message)?
            .into_iter()
            .find(|entry| entry.id == distributionId)
            .ok_or("Terminal distribution is not registered")?;
        let scheduler = self
            .context
            .hostRuntimeTaskSchedulerHost
            .clone()
            .ok_or("Terminal installation requires a host task scheduler")?;
        let installId = format!("rootfs_{}", uuid::Uuid::new_v4().simple());
        let snapshot = RuntimeTerminalInstall {
            installId: installId.clone(),
            distributionId: distributionId.clone(),
            name: name.trim().into(),
            state: "queued".into(),
            downloadedBytes: 0,
            totalBytes: distribution.downloadBytes,
            profileId: None,
            error: None,
        };
        let control = operit_host_api::HttpDownloadControl::new();
        {
            let mut jobs = terminal_install_jobs()
                .lock()
                .map_err(|_| "Terminal installation registry is poisoned")?;
            jobs.retain(|job| job.host.strong_count() > 0);
            let host = Arc::downgrade(&self.terminalHost);
            if jobs.iter().any(|job| {
                std::sync::Weak::ptr_eq(&job.host, &host)
                    && job.snapshot.distributionId == distributionId
                    && !job.isFinished()
            }) {
                return Err("This terminal distribution is already being installed".into());
            }
            if jobs.iter().filter(|job| !job.isFinished()).count() >= 4 {
                return Err("Too many active terminal installations".into());
            }
            if jobs.len() >= 32 {
                if let Some(index) = jobs.iter().position(TerminalInstallJob::isFinished) {
                    jobs.remove(index);
                }
            }
            jobs.push(TerminalInstallJob {
                host,
                control: control.clone(),
                snapshot: snapshot.clone(),
            });
        }
        let progressId = installId.clone();
        let progress: TerminalInstallProgressCallback = Arc::new(move |value| {
            let mut jobs = terminal_install_jobs()
                .lock()
                .expect("Terminal installation registry is poisoned");
            if let Some(job) = jobs.iter_mut().find(|job| job.snapshot.installId == progressId) {
                job.snapshot.state = value.state;
                job.snapshot.downloadedBytes = value.downloadedBytes;
                job.snapshot.totalBytes = value.totalBytes;
            }
        });
        let host = self.terminalHost.clone();
        let workerId = installId.clone();
        let result = scheduler.scheduleHostRuntimeTask(
            "install-terminal-distribution",
            Box::new(move || {
                let result = host.installTerminalDistribution(
                    &distributionId,
                    &workerId,
                    &name,
                    control.clone(),
                    progress,
                );
                let mut jobs = terminal_install_jobs()
                    .lock()
                    .expect("Terminal installation registry is poisoned");
                if let Some(job) = jobs.iter_mut().find(|job| job.snapshot.installId == workerId) {
                    match result {
                        Ok(profile) => {
                            job.snapshot.state = "completed".into();
                            job.snapshot.profileId = Some(profile.id);
                        }
                        Err(error) => {
                            job.snapshot.state = if control.isCancelled() {
                                "cancelled"
                            } else {
                                "failed"
                            }
                            .into();
                            job.snapshot.error = Some(error.message);
                        }
                    }
                }
            }),
        );
        if let Err(error) = result {
            terminal_install_jobs()
                .lock()
                .map_err(|_| "Terminal installation registry is poisoned")?
                .retain(|job| job.snapshot.installId != installId);
            return Err(error.message);
        }
        Ok(snapshot)
    }

    /// Reads this host's installation attempts in start order, including terminal errors.
    pub fn terminalDistributionInstalls(&self) -> Result<Vec<RuntimeTerminalInstall>, String> {
        let jobs = terminal_install_jobs()
            .lock()
            .map_err(|_| "Terminal installation registry is poisoned")?;
        Ok(jobs
            .iter()
            .filter(|job| std::sync::Weak::ptr_eq(&job.host, &Arc::downgrade(&self.terminalHost)))
            .map(|job| job.snapshot.clone())
            .collect())
    }

    /// Requests cancellation while retaining the job until download and staging cleanup finish.
    pub fn cancelTerminalDistributionInstall(&self, installId: String) -> Result<(), String> {
        let jobs = terminal_install_jobs()
            .lock()
            .map_err(|_| "Terminal installation registry is poisoned")?;
        let job = jobs
            .iter()
            .find(|job| job.snapshot.installId == installId)
            .filter(|job| std::sync::Weak::ptr_eq(&job.host, &Arc::downgrade(&self.terminalHost)))
            .ok_or("Terminal installation does not exist")?;
        if job.isFinished() {
            return Err("Terminal installation has already finished".into());
        }
        job.control.cancel();
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Starts a PTY on the host's blocking pool without holding the Core proxy lock.
    pub fn startTerminalPty(
        &self,
        sessionName: String,
        terminal: String,
        terminalType: String,
        workingDir: String,
        rows: i32,
        cols: i32,
    ) -> impl Future<Output = Result<String, String>> + 'static {
        let service = Self {
            terminalHost: self.terminalHost.clone(),
            context: self.context.clone(),
        };
        async move {
            let scheduler = service
                .context
                .hostRuntimeTaskSchedulerHost
                .clone()
                .ok_or_else(|| {
                    "Starting a terminal requires a runtime task scheduler".to_string()
                })?;
            let (send, receive) = tokio::sync::oneshot::channel();
            scheduler
                .scheduleHostRuntimeTask(
                    "start-terminal-pty",
                    Box::new(move || {
                        let result = service.start_terminal_pty(
                            sessionName,
                            terminal,
                            terminalType,
                            workingDir,
                            rows,
                            cols,
                        );
                        if let Err(Ok(session_id)) = send.send(result) {
                            let _ = service.closeTerminalPty(session_id);
                        }
                    }),
                )
                .map_err(|error| error.message)?;
            receive
                .await
                .map_err(|_| "Terminal startup worker stopped".to_string())?
        }
    }

    #[allow(non_snake_case)]
    /// Resolves the working directory, starts the host PTY, and publishes it.
    fn start_terminal_pty(
        &self,
        sessionName: String,
        terminal: String,
        terminalType: String,
        workingDir: String,
        rows: i32,
        cols: i32,
    ) -> Result<String, String> {
        let resolvedWorkingDir = self
            .terminalHost
            .resolveWorkingDirectory(
                &terminal,
                &terminalType,
                &workingDir,
                &|path| resolve_terminal_working_dir(&self.context, self.terminalHost.as_ref(), path)
                    .map_err(operit_host_api::HostError::new),
            )
            .map_err(|error| format!(
                "Cannot resolve terminal working directory for {terminal}/{terminalType} at {workingDir}: {}",
                error.message,
            ))?;
        let sessionId = self
            .terminalHost
            .startPtySession(
                &sessionName,
                &terminal,
                &terminalType,
                &resolvedWorkingDir,
                rows as u16,
                cols as u16,
            )
            .map_err(|error| error.message)?;
        if let Err(error) = operit_host_api::TerminalWorkingDirectory::enterTerminalDirectory(
            self.terminalHost.as_ref(),
            &sessionId,
            &terminalType,
            &resolvedWorkingDir,
        ) {
            let _ = self.terminalHost.closePtySession(&sessionId);
            return Err(error.message);
        }
        self.ensureTerminalPtyOutputStream(sessionId.clone());
        publish_terminal_sessions(self.terminalHost.as_ref())?;
        Ok(sessionId)
    }

    #[allow(non_snake_case)]
    /// Returns the shared output stream for a PTY session.
    pub fn terminalPtyOutput(&self, sessionId: String) -> RuntimeTerminalPtyOutputStream {
        RuntimeTerminalPtyOutputStream::new(self.ensureTerminalPtyOutputStream(sessionId))
    }

    #[allow(non_snake_case)]
    /// Writes base64-encoded bytes to a PTY session.
    pub fn writeTerminalPty(&self, sessionId: String, dataBase64: String) -> Result<i32, String> {
        let data = STANDARD
            .decode(dataBase64.as_bytes())
            .map_err(|error| error.to_string())?;
        self.terminalHost
            .writePtySession(&sessionId, &data)
            .map(|count| count as i32)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Resizes a PTY session.
    pub fn resizeTerminalPty(&self, sessionId: String, rows: i32, cols: i32) -> Result<(), String> {
        self.terminalHost
            .resizePtySession(&sessionId, rows as u16, cols as u16)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Polls the exit code for a PTY session.
    pub fn pollTerminalPtyExit(&self, sessionId: String) -> Result<Option<i32>, String> {
        self.terminalHost
            .pollPtyExitCode(&sessionId)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Closes a PTY session and removes its output stream.
    pub fn closeTerminalPty(&self, sessionId: String) -> Result<(), String> {
        close_terminal_pty_output_stream(&sessionId);
        self.terminalHost
            .closePtySession(&sessionId)
            .map_err(|error| error.message)?;
        publish_terminal_sessions(self.terminalHost.as_ref())?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Sends text input to a terminal session.
    pub fn inputTerminalSession(&self, sessionId: String, input: String) -> Result<i32, String> {
        self.terminalHost
            .inputInSession(&sessionId, Some(&input), None)
            .map(|output| output.acceptedChars as i32)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Reads the current screen contents for a terminal session.
    pub fn getTerminalSessionScreen(
        &self,
        sessionId: String,
    ) -> Result<RuntimeTerminalScreen, String> {
        self.terminalHost
            .getSessionScreen(&sessionId)
            .map_err(|error| error.message)
            .map(|screen| RuntimeTerminalScreen {
                sessionId: screen.sessionId,
                platform: screen.platform,
                terminal: screen.terminal,
                terminalType: screen.terminalType,
                rows: screen.rows as i32,
                cols: screen.cols as i32,
                content: screen.content,
                commandRunning: screen.commandRunning,
            })
    }

    #[allow(non_snake_case)]
    fn ensureTerminalPtyOutputStream(&self, sessionId: String) -> MutableSharedStreamImpl<String> {
        let mut streams = terminal_pty_output_streams()
            .lock()
            .expect("terminal pty output streams mutex poisoned");
        if let Some(entry) = streams.get(&sessionId) {
            return entry.stream.clone();
        }
        let stream = MutableSharedStreamImpl::new(512);
        streams.insert(
            sessionId.clone(),
            TerminalPtyOutputEntry {
                stream: stream.clone(),
            },
        );
        let taskScheduler = self
            .context
            .hostRuntimeTaskSchedulerHost
            .clone()
            .expect("RuntimeTerminalService requires a HostRuntimeTaskSchedulerHost");
        start_terminal_pty_output_reader(
            self.terminalHost.clone(),
            taskScheduler,
            sessionId,
            stream.clone(),
        );
        stream
    }
}

/// Resolves host-owned directories after the selected terminal has dispatched its namespace.
fn resolve_terminal_working_dir(
    context: &HostManager,
    terminalHost: &dyn TerminalHost,
    workingDir: &str,
) -> Result<String, String> {
    let filesystem = terminal_vfs(context)?;
    let path = filesystem.resolveResourcePath(None, workingDir.trim())?;
    filesystem.terminalDirectory(terminalHost, &path)
}

/// Builds the host VFS used only for directories owned by the current runtime.
fn terminal_vfs(context: &HostManager) -> Result<VisualFileSystem, String> {
    context.vfs()
}

#[cfg(test)]
mod idle_poll_tests {
    use super::*;
    use operit_host_api::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestTerminal {
        reads: AtomicUsize,
        exited: bool,
    }
    #[allow(unused_variables)]
    impl TerminalHost for TestTerminal {
        /// Resolves host directories without changing this stream-test host's capabilities.
        fn resolveWorkingDirectory(
            &self,
            _: &str,
            _: &str,
            workingDir: &str,
            resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
        ) -> HostResult<String> {
            resolveHostDirectory(workingDir)
        }
        fn terminalInfo(&self) -> HostResult<TerminalInfo> {
            unimplemented!()
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
            unimplemented!()
        }
        fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
            let read = self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(if self.exited {
                if read == 0 {
                    b"before".to_vec()
                } else {
                    b"tail".to_vec()
                }
            } else {
                Vec::new()
            })
        }
        fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
            unimplemented!()
        }
        fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
            unimplemented!()
        }
        fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
            Ok(if self.exited { Some(0) } else { None })
        }
        fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
            unimplemented!()
        }
        fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
            Ok(Vec::new())
        }
        fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
            unimplemented!()
        }
        /// Rejects typed session creation in this output-stream-only test host.
        fn createOrGetTypedSession(&self, _: &str, _: &str) -> HostResult<TerminalSessionInfo> {
            unimplemented!()
        }
        fn executeInSession(
            &self,
            sessionId: &str,
            command: &str,
            timeoutMs: u64,
        ) -> HostResult<TerminalCommandOutput> {
            unimplemented!()
        }
        fn executeHiddenCommand(
            &self,
            command: &str,
            executorKey: &str,
            timeoutMs: u64,
        ) -> HostResult<HiddenTerminalCommandOutput> {
            unimplemented!()
        }
        fn inputInSession(
            &self,
            sessionId: &str,
            input: Option<&str>,
            control: Option<&str>,
        ) -> HostResult<TerminalInputOutput> {
            unimplemented!()
        }
        fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
            unimplemented!()
        }
        fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
            unimplemented!()
        }
    }

    fn register(id: &str) -> MutableSharedStreamImpl<String> {
        let stream = MutableSharedStreamImpl::new(8);
        terminal_pty_output_streams().lock().unwrap().insert(
            id.to_owned(),
            TerminalPtyOutputEntry {
                stream: stream.clone(),
            },
        );
        stream
    }

    #[test]
    fn closed_reader_stops_without_touching_host() {
        let id = "idle-poll-test-close";
        let stream = register(id);
        let host = TestTerminal {
            reads: AtomicUsize::new(0),
            exited: false,
        };
        assert!(poll_terminal_pty_output(&host, id, &stream));
        close_terminal_pty_output_stream(id);
        assert!(!poll_terminal_pty_output(&host, id, &stream));
        assert_eq!(host.reads.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn exit_drains_final_bytes_before_closing() {
        let id = "idle-poll-test-exit";
        let stream = register(id);
        let host = TestTerminal {
            reads: AtomicUsize::new(0),
            exited: true,
        };
        assert!(!poll_terminal_pty_output(&host, id, &stream));
        assert_eq!(
            stream.replay_cache(),
            vec![STANDARD.encode(b"before"), STANDARD.encode(b"tail")]
        );
        assert!(!poll_terminal_pty_output(&host, id, &stream));
        assert_eq!(host.reads.load(Ordering::SeqCst), 2);
    }
}
