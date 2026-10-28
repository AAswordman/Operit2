//! Headless Core owner: single-instance lock, daemon lifecycle, and the
//! local control channel shared by the daemon and a running TUI.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use operit_host_native_common::DaemonHost::ControlStream;
use operit_node_runtime::NodeServices::PeerTransport;
use operit_proxy_local::{GeneratedCoreProxy, LocalCoreProxy};
use operit_util::AppLogger::AppLogger;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};

use crate::bootstrap::{active_cli_identity, CliIdentity};
use crate::core_proxy::SharedLocalCore;
use crate::create_cli_core_application;
use crate::host::daemon::{self as DaemonHost, ControlListener};

/// One control request or response; a command line must stay below this cap.
const MAX_CONTROL_BYTES: usize = 16 * 1024 * 1024;
/// Size cap for the detached daemon output file.
const DAEMON_OUT_MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Newest bytes kept when the detached daemon output file is trimmed.
const DAEMON_OUT_KEEP_BYTES: usize = 1024 * 1024;
/// A routed command may wait for a model response for a long time.
const CONTROL_READ_TIMEOUT: Duration = Duration::from_secs(3600);
const CONTROL_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const DAEMON_READY_TIMEOUT: Duration = Duration::from_secs(60);

/// Whether this process owns the identity's Core. Short-lived CLI Cores must
/// use a different plugin SDK IPC instance so they do not fight the owner for
/// the same bus name.
static OWNER_PROCESS: AtomicBool = AtomicBool::new(false);

/// Reports whether this process owns the active identity Core.
pub(crate) fn process_is_owner() -> bool {
    OWNER_PROCESS.load(Ordering::Relaxed)
}

/// Reports whether another process (or this one) currently holds the owner
/// lock. Used to keep short-lived CLI Cores from claiming the sync worker.
pub(crate) fn owner_is_running() -> bool {
    let Ok(paths) = DaemonPaths::for_active_identity() else {
        return false;
    };
    owner_lock_held(&paths)
}

/// Runtime state shared by the status command, the control channel, and the
/// TUI status line.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DaemonStatus {
    pub pid: u32,
    /// `daemon` or `tui`.
    pub mode: String,
    /// `starting` until Core startup and socket binding complete, then `ready`.
    pub state: String,
    pub identity_id: String,
    pub identity_name: String,
    pub started_at: u64,
    pub socket: String,
    pub log_path: String,
    pub output_log: String,
    pub version: String,
    /// Runtime-only counter for commands routed through the control channel.
    #[serde(default)]
    pub commands: u64,
}

/// Identity-scoped daemon paths. The socket lives outside the identity
/// directory because Unix socket paths have a short length limit.
#[derive(Clone, Debug)]
pub(crate) struct DaemonPaths {
    pub identity_id: String,
    pub identity_name: String,
    pub daemon_dir: PathBuf,
    pub lock_path: PathBuf,
    pub status_path: PathBuf,
    pub socket_path: PathBuf,
    pub output_log_path: PathBuf,
}

impl DaemonPaths {
    /// Resolves daemon paths for the selected CLI identity.
    pub(crate) fn for_active_identity() -> Result<Self, String> {
        let (identity, runtime_root, _) = active_cli_identity();
        Ok(Self::at(&runtime_root, &identity))
    }

    /// Builds persistent paths and the host-owned endpoint for one identity.
    fn at(identity_root: &Path, identity: &CliIdentity) -> Self {
        let daemon_dir = identity_root.join("daemon");
        let hex = format!(
            "{:x}",
            Sha256::digest(identity_root.as_os_str().as_encoded_bytes())
        );
        let socket_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|path| path.is_dir())
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("operit2-{}", &hex[..8]));
        Self {
            identity_id: identity.id.clone(),
            identity_name: identity.name.clone(),
            lock_path: daemon_dir.join("owner.lock"),
            status_path: daemon_dir.join("status.json"),
            socket_path: DaemonHost::control_endpoint(&socket_dir, &hex),
            output_log_path: identity_root.join("logs").join("daemon.out"),
            daemon_dir,
        }
    }
}

/// Holds the exclusive owner lock for one identity. The daemon and the TUI
/// must not run at the same time: both would own the same sync worker and
/// listener state.
pub(crate) struct OwnerGuard {
    paths: DaemonPaths,
    _lock: File,
    status: DaemonStatus,
}

impl OwnerGuard {
    /// Acquires exclusive Core ownership for the active identity.
    pub(crate) fn acquire(mode: &str) -> Result<Self, String> {
        Self::acquire_at(DaemonPaths::for_active_identity()?, mode)
    }

    /// Locks one identity before publishing its initial owner status.
    fn acquire_at(paths: DaemonPaths, mode: &str) -> Result<Self, String> {
        ensure_private_dir(&paths.daemon_dir)?;
        if let Some(parent) = paths.output_log_path.parent() {
            ensure_private_dir(parent)?;
        }
        let mut options = DaemonHost::private_open_options();
        options.read(true).write(true).create(true);
        let lock = options
            .open(&paths.lock_path)
            .map_err(|error| format!("cannot open {}: {error}", paths.lock_path.display()))?;
        match lock.try_lock_exclusive() {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
                let detail = read_status(&paths.status_path)
                    .map(|status| {
                        format!(
                            " (pid {}, mode {}, state {})",
                            status.pid, status.mode, status.state
                        )
                    })
                    .unwrap_or_default();
                return Err(format!(
                    "another Core owner is already running for identity '{}'{detail}; \
                     stop it with 'operit2 daemon stop' or close the TUI",
                    paths.identity_name
                ));
            }
            Err(error) => {
                return Err(format!(
                    "cannot lock {}: {error}",
                    paths.lock_path.display()
                ))
            }
        }
        let status = DaemonStatus {
            pid: std::process::id(),
            mode: mode.to_string(),
            state: "starting".to_string(),
            identity_id: paths.identity_id.clone(),
            identity_name: paths.identity_name.clone(),
            started_at: now_millis(),
            socket: paths.socket_path.display().to_string(),
            log_path: AppLogger::get_log_file_path().unwrap_or_default(),
            output_log: paths.output_log_path.display().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            commands: 0,
        };
        write_status(&paths.status_path, &status)?;
        OWNER_PROCESS.store(true, Ordering::Relaxed);
        Ok(Self {
            paths,
            _lock: lock,
            status,
        })
    }

    /// Returns the paths protected by this owner lock.
    pub(crate) fn paths(&self) -> &DaemonPaths {
        &self.paths
    }

    /// Returns the latest persisted owner status.
    pub(crate) fn status(&self) -> &DaemonStatus {
        &self.status
    }

    /// Captures the runtime log path once Core startup has initialized the
    /// logger; before that AppLogger has no file yet.
    pub(crate) fn refresh_log_path(&mut self) -> Result<(), String> {
        self.status.log_path = AppLogger::get_log_file_path().unwrap_or_default();
        write_status(&self.paths.status_path, &self.status)
    }

    /// Publishes readiness after the host control endpoint has bound.
    pub(crate) fn mark_ready(&mut self) -> Result<(), String> {
        self.status.state = "ready".to_string();
        write_status(&self.paths.status_path, &self.status)
    }
}

impl Drop for OwnerGuard {
    /// Removes owner metadata before releasing the identity lock.
    fn drop(&mut self) {
        OWNER_PROCESS.store(false, Ordering::Relaxed);
        let _ = DaemonHost::cleanup_endpoint(&self.paths.socket_path);
        let _ = fs::remove_file(&self.paths.status_path);
    }
}

#[derive(Serialize, Deserialize)]
struct ControlRequest {
    op: String,
    #[serde(default)]
    argv: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<DaemonStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ControlResponse {
    /// Builds a successful response without an output payload.
    fn ok() -> Self {
        Self {
            ok: true,
            stdout: None,
            stderr: None,
            status: None,
            error: None,
        }
    }

    /// Builds a response containing the command output.
    fn output(stdout: String, stderr: String) -> Self {
        Self {
            ok: true,
            stdout: Some(stdout),
            stderr: Some(stderr),
            status: None,
            error: None,
        }
    }

    /// Builds a response containing the live owner status.
    fn status(status: DaemonStatus) -> Self {
        Self {
            ok: true,
            stdout: None,
            stderr: None,
            status: Some(status),
            error: None,
        }
    }

    /// Builds a failed response with its original error.
    fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            stdout: None,
            stderr: None,
            status: None,
            error: Some(error.into()),
        }
    }
}

/// Accepts control connections while the owning Core is alive. Every command
/// gets its own generated proxy over the same local Core handle.
pub(crate) struct ControlServer {
    listener: ControlListener,
    client: Arc<LocalCoreProxy>,
    status: Arc<DaemonStatus>,
    commands: Arc<AtomicU64>,
    shutdown: watch::Sender<bool>,
}

impl ControlServer {
    /// Binds the host control endpoint before publishing the owner's ready status.
    pub(crate) fn bind(
        owner: &mut OwnerGuard,
        client: Arc<LocalCoreProxy>,
    ) -> Result<Self, String> {
        let path = &owner.paths().socket_path;
        let listener = ControlListener::bind(path)
            .map_err(|error| format!("cannot bind {}: {error}", path.display()))?;
        owner.mark_ready()?;
        let (shutdown, _) = watch::channel(false);
        Ok(Self {
            listener,
            client,
            status: Arc::new(owner.status().clone()),
            commands: Arc::new(AtomicU64::new(0)),
            shutdown,
        })
    }

    /// Shares a persistent shutdown state with the owner and the accept loop.
    pub(crate) fn shutdown_token(&self) -> watch::Sender<bool> {
        self.shutdown.clone()
    }

    /// Spawns the accept loop on the current LocalSet; generated Core futures
    /// are not Send, so this must not use the multi-threaded spawn.
    pub(crate) fn spawn_local(self) -> JoinHandle<()> {
        tokio::task::spawn_local(async move {
            self.serve().await;
        })
    }

    /// Accepts control requests until the shared shutdown state is set.
    async fn serve(self) {
        let Self {
            mut listener,
            client,
            status,
            commands,
            shutdown,
        } = self;
        let mut stopped = shutdown.subscribe();
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                biased;
                _ = stopped.wait_for(|value| *value) => break,
                accepted = listener.accept() => {
                    match accepted {
                        Ok(stream) => {
                            let client = client.clone();
                            let status = status.clone();
                            let commands = commands.clone();
                            let shutdown = shutdown.clone();
                            connections.spawn_local(async move {
                                handle_connection(stream, client, status, commands, shutdown).await;
                            });
                        }
                        Err(error) => {
                            AppLogger::e(
                                "DaemonControl",
                                &format!("control channel accept failed: {error}"),
                            );
                            break;
                        }
                    }
                }
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
            }
        }
        let drain = async { while connections.join_next().await.is_some() {} };
        if tokio::time::timeout(Duration::from_secs(3), drain)
            .await
            .is_err()
        {
            connections.abort_all();
        }
    }
}

/// Handles one framed request over the host control stream.
async fn handle_connection(
    stream: ControlStream,
    client: Arc<LocalCoreProxy>,
    status: Arc<DaemonStatus>,
    commands: Arc<AtomicU64>,
    shutdown: watch::Sender<bool>,
) {
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    let response = match tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
        .await
    {
        Ok(Ok(0)) => ControlResponse::error("control request was empty"),
        Ok(Ok(_)) if line.len() > MAX_CONTROL_BYTES => {
            ControlResponse::error("control request is too large")
        }
        Ok(Ok(_)) => match serde_json::from_str::<ControlRequest>(&line) {
            Ok(request) => handle_request(request, client, status, commands, shutdown).await,
            Err(error) => ControlResponse::error(format!("invalid control request: {error}")),
        },
        Ok(Err(error)) => ControlResponse::error(format!("control request read failed: {error}")),
        Err(_) => ControlResponse::error("control request timed out"),
    };
    let text = serde_json::to_string(&response).unwrap_or_else(|_| {
        "{\"ok\":false,\"error\":\"response serialization failed\"}".to_string()
    });
    let text = if text.len() <= MAX_CONTROL_BYTES {
        text
    } else {
        serde_json::to_string(&ControlResponse::error(
            "command output is too large to return over the control channel",
        ))
        .expect("error response must serialize")
    };
    let _ = write_half.write_all(text.as_bytes()).await;
    let _ = write_half.write_all(b"\n").await;
    let _ = write_half.flush().await;
}

/// Dispatches owner operations and commands against the shared Core.
async fn handle_request(
    request: ControlRequest,
    client: Arc<LocalCoreProxy>,
    status: Arc<DaemonStatus>,
    commands: Arc<AtomicU64>,
    shutdown: watch::Sender<bool>,
) -> ControlResponse {
    match request.op.as_str() {
        "ping" => ControlResponse::ok(),
        "status" => {
            let mut status = (*status).clone();
            status.commands = commands.load(Ordering::Relaxed);
            ControlResponse::status(status)
        }
        "shutdown" => {
            if status.mode != "daemon" {
                return ControlResponse::error("the TUI owns this identity; quit it from the TUI");
            }
            shutdown.send_replace(true);
            ControlResponse::ok()
        }
        "peer-connection-code" => {
            let mut proxy = GeneratedCoreProxy::new(SharedLocalCore(client));
            match proxy.server_runtime_remote_link_service().peerConnectionCode().await {
                Ok(code) => ControlResponse::output(code, String::new()),
                Err(error) => ControlResponse::error(error.to_string()),
            }
        }
        "run" => {
            if request.argv.is_empty() {
                return ControlResponse::error("run requires argv");
            }
            commands.fetch_add(1, Ordering::Relaxed);
            let mut proxy = GeneratedCoreProxy::new(SharedLocalCore(client));
            match proxy.runCoreCommand(&request.argv).await {
                Ok(output) => ControlResponse::output(output.stdout, output.stderr),
                Err(error) => ControlResponse::error(error.to_string()),
            }
        }
        other => ControlResponse::error(format!("unknown control operation: {other}")),
    }
}

/// Reads the live owner's listener coordinates. A failed owner is never replaced by a second Core.
pub(crate) async fn owner_peer_connection_code() -> Option<Result<String, String>> {
    let paths = match DaemonPaths::for_active_identity() {
        Ok(paths) => paths,
        Err(error) => return Some(Err(error)),
    };
    if !owner_lock_held(&paths) { return None; }
    Some(async {
        let response = request_at_with_timeout(&paths.socket_path, &ControlRequest {
            op: "peer-connection-code".into(), argv: Vec::new(),
        }, Duration::from_secs(8)).await?;
        if !response.ok { return Err(response.error.unwrap_or_else(|| "Core could not share a connection code".into())); }
        response.stdout.filter(|code| !code.is_empty()).ok_or_else(|| "Core returned no connection code".into())
    }.await)
}

/// Runs CLI commands through an owning daemon/TUI when one is available.
/// Returns `None` when no owner exists and the caller should boot its own
/// short-lived Core as before.
pub(crate) async fn try_run_cli_command(args: &[String], json: bool) -> Option<Result<(), String>> {
    if !route_supported(args) {
        return None;
    }
    let paths = match DaemonPaths::for_active_identity() {
        Ok(paths) => paths,
        Err(error) => return Some(Err(error)),
    };
    let mut argv = args.to_vec();
    if json {
        argv.push("--json".to_string());
    }
    let request = ControlRequest {
        op: "run".to_string(),
        argv,
    };
    let stream = match connect_with_retry(&paths).await {
        Some(stream) => stream,
        None => {
            if owner_lock_held(&paths) {
                return Some(Err(format!(
                    "a Core owner is starting or unresponsive for identity '{}'; retry shortly \
                     or run 'operit2 daemon status'",
                    paths.identity_name
                )));
            }
            return None;
        }
    };
    match exchange(stream, &request).await {
        Ok(response) => Some(print_response(response, json)),
        Err(error) => Some(Err(format!("control command failed: {error}"))),
    }
}

/// Commands that must run in the invoking process: interactive shells, local
/// storage/OAuth flows, and link operations that own their own listener.
fn route_supported(args: &[String]) -> bool {
    let Some(command) = args.first().map(String::as_str) else {
        return false;
    };
    match command {
        "model" => args.get(1).map(String::as_str) != Some("codex-login"),
        "chat" => args.get(1).map(String::as_str) != Some("shell"),
        "version" | "storage" | "tts" | "stt" | "export" | "import" | "backup" | "market"
        | "update" | "shell" | "web" | "link" | "identity" | "install" | "uninstall" | "rescue"
        | "host" => false,
        _ => true,
    }
}

/// Waits for the locked owner to finish binding its control endpoint.
async fn connect_with_retry(paths: &DaemonPaths) -> Option<ControlStream> {
    for _ in 0..20 {
        if !owner_lock_held(paths) {
            return None;
        }
        if let Ok(Ok(stream)) = tokio::time::timeout(
            CONTROL_CONNECT_TIMEOUT,
            DaemonHost::connect(&paths.socket_path),
        )
        .await
        {
            return Some(stream);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    None
}

/// Connects to one host endpoint and enforces response deadlines.
async fn request_at_with_timeout(
    path: &Path,
    request: &ControlRequest,
    read_timeout: Duration,
) -> Result<ControlResponse, String> {
    let stream = tokio::time::timeout(CONTROL_CONNECT_TIMEOUT, DaemonHost::connect(path))
        .await
        .map_err(|_| "control channel connect timed out".to_string())?
        .map_err(|error| error.to_string())?;
    exchange_with_timeout(stream, request, read_timeout).await
}

/// Requests live owner status over its control channel.
async fn status_request(path: &Path) -> Result<ControlResponse, String> {
    request_at_with_timeout(
        path,
        &ControlRequest {
            op: "status".to_string(),
            argv: Vec::new(),
        },
        Duration::from_secs(2),
    )
    .await
}

/// Exchanges one command using the normal command response deadline.
async fn exchange(
    stream: ControlStream,
    request: &ControlRequest,
) -> Result<ControlResponse, String> {
    exchange_with_timeout(stream, request, CONTROL_READ_TIMEOUT).await
}

/// Writes one JSON request and reads its bounded JSON response.
async fn exchange_with_timeout(
    mut stream: ControlStream,
    request: &ControlRequest,
    read_timeout: Duration,
) -> Result<ControlResponse, String> {
    let mut line = serde_json::to_string(request).map_err(|error| error.to_string())?;
    line.push('\n');
    stream
        .write_all(line.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    stream.flush().await.map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    let read = tokio::time::timeout(read_timeout, reader.read_line(&mut response))
        .await
        .map_err(|_| "control channel response timed out".to_string())?
        .map_err(|error| error.to_string())?;
    if read == 0 {
        return Err("control channel closed without a response".to_string());
    }
    if response.len() > MAX_CONTROL_BYTES {
        return Err("control response is too large".to_string());
    }
    serde_json::from_str(&response).map_err(|error| format!("invalid control response: {error}"))
}

/// Prints the returned command output and preserves errors.
fn print_response(response: ControlResponse, json: bool) -> Result<(), String> {
    if let Some(stdout) = response.stdout {
        if !stdout.is_empty() {
            if json {
                println!("{stdout}");
            } else {
                print!("{}", crate::cli::rewrite_core_command_usage_message(stdout));
            }
        }
    }
    if !json {
        if let Some(stderr) = response.stderr {
            if !stderr.is_empty() {
                eprint!("{}", crate::cli::rewrite_core_command_usage_message(stderr));
            }
        }
    }
    if response.ok {
        Ok(())
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "control command failed".to_string()))
    }
}

/// Checks whether an existing owner holds the identity lock.
fn owner_lock_held(paths: &DaemonPaths) -> bool {
    let Ok(file) = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&paths.lock_path)
    else {
        return false;
    };
    match file.try_lock_shared() {
        Ok(()) => {
            let _ = FileExt::unlock(&file);
            false
        }
        Err(_) => true,
    }
}

/// Distinguishes stopped, starting, and responsive ready owners.
async fn query_status_at(paths: &DaemonPaths) -> Result<Option<DaemonStatus>, String> {
    if !owner_lock_held(paths) {
        return Ok(None);
    }
    let bytes = match fs::read(&paths.status_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read owner status: {error}")),
    };
    let status: DaemonStatus =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid owner status: {error}"))?;
    if status.state == "starting" {
        return Ok(Some(status));
    }
    match status_request(&paths.socket_path).await {
        Ok(response) if response.ok => Ok(response.status),
        Ok(response) => Err(response
            .error
            .unwrap_or_else(|| "status request failed".to_string())),
        Err(error) => {
            if owner_lock_held(paths) {
                Err(format!("control channel is unresponsive: {error}"))
            } else {
                Ok(None)
            }
        }
    }
}

/// Root entry point for `operit2 daemon ...`.
pub(crate) async fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("run") => run_foreground(&args[1..]).await,
        Some("start") => start_background(&args[1..]).await,
        Some("status") => status_command(args.iter().any(|arg| arg == "--json")).await,
        Some("stop") => stop_command(args.iter().any(|arg| arg == "--json")).await,
        Some("help" | "-h" | "--help") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => Err(format!(
            "unknown daemon command: {other}; run 'operit2 daemon help'"
        )),
    }
}

/// Prints the supported daemon commands.
fn print_usage() {
    println!("operit2 daemon run [--no-listen] [--link-listen <transport>[,<transport>...]]");
    println!("operit2 daemon start [--no-listen] [--link-listen <transport>[,<transport>...]]");
    println!("operit2 daemon status [--json]");
    println!("operit2 daemon stop [--json]");
    println!();
    println!("run      Run in the foreground (use this under systemd).");
    println!("start    Spawn a detached daemon and wait until it is ready.");
    println!("status   Show the current Core owner (daemon or TUI).");
    println!("stop     Ask a running daemon to shut down.");
    println!();
    println!("Routed `operit2 cli ...` commands use the owner's control channel.");
}

#[derive(Default)]
struct RunOptions {
    no_listen: bool,
    listen: Vec<PeerTransport>,
}

impl RunOptions {
    /// Parses listener options without changing identity configuration.
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--no-listen" => {
                    options.no_listen = true;
                    index += 1;
                }
                "--link-listen" => {
                    let value = args.get(index + 1).ok_or_else(|| {
                        "usage: operit2 daemon run [--no-listen] --link-listen <http|ws|tcp|serial|bluetooth>[,<transport>...]".to_string()
                    })?;
                    for spelling in value.split(',') {
                        let transport = parse_transport(spelling.trim())?;
                        if !options.listen.contains(&transport) {
                            options.listen.push(transport);
                        }
                    }
                    index += 2;
                }
                other => return Err(format!("unknown daemon option: {other}")),
            }
        }
        Ok(options)
    }
}

/// Resolves one explicitly supported listener transport.
fn parse_transport(value: &str) -> Result<PeerTransport, String> {
    match value {
        "http" => Ok(PeerTransport::Http),
        "ws" => Ok(PeerTransport::WebSocket),
        "tcp" => Ok(PeerTransport::Tcp),
        "serial" => Ok(PeerTransport::Serial),
        "bluetooth" => Ok(PeerTransport::Bluetooth),
        _ => Err(format!(
            "unknown transport: {value}; expected http, ws, tcp, serial or bluetooth"
        )),
    }
}

/// Owns a Core until a control or host shutdown request arrives.
async fn run_foreground(args: &[String]) -> Result<(), String> {
    let options = RunOptions::parse(args)?;
    let mut owner = OwnerGuard::acquire("daemon")?;
    // Keep the historical "client" role so the default device label stays
    // stable when the same identity switches between TUI and daemon.
    let core_application = create_cli_core_application("client").await?;
    let result = async {
        start_listeners(&core_application, &options).await?;
        owner.refresh_log_path()?;
        let server = ControlServer::bind(&mut owner, core_application.localClient())?;
        AppLogger::i(
            "Daemon",
            &format!(
                "daemon ready pid={} socket={}",
                std::process::id(),
                owner.paths().socket_path.display()
            ),
        );
        println!(
            "daemon ready (pid {}, socket {})",
            std::process::id(),
            owner.paths().socket_path.display()
        );
        let shutdown = server.shutdown_token();
        let mut stopped = shutdown.subscribe();

        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let mut server_task = server.spawn_local();
                let outcome = tokio::select! {
                    biased;
                    _ = stopped.wait_for(|value| *value) => Ok(()),
                    signal = tokio::signal::ctrl_c() => signal.map_err(|error| error.to_string()),
                    signal = DaemonHost::termination_signal() => signal.map_err(|error| error.to_string()),
                    result = &mut server_task => {
                        return match result {
                            Ok(()) => Err("daemon control server stopped unexpectedly".to_string()),
                            Err(error) => Err(format!("daemon control server failed: {error}")),
                        };
                    }
                };
                // Let the accept loop drain the shutdown response before dropping it.
                shutdown.send_replace(true);
                let _ = tokio::time::timeout(Duration::from_secs(5), server_task).await;
                outcome
            })
            .await
    }
    .await;

    core_application.shutdown().await;
    AppLogger::i("Daemon", "daemon stopped");
    drop(owner);
    result
}

/// Starts the listeners selected by explicit or saved configuration.
async fn start_listeners(
    core_application: &operit_core_application::CoreApplication,
    options: &RunOptions,
) -> Result<(), String> {
    if options.no_listen {
        return Ok(());
    }
    let services = core_application.accessServices();
    let explicit = !options.listen.is_empty();
    let transports = if explicit {
        let mut config = services.localHostConfig()?.ok_or("Node listener preferences are not initialized")?;
        config.transports = options.listen.clone();
        config.updatedAt = operit_host_api::TimeUtils::currentTimeMillis();
        services.saveLocalHostConfig(config)?;
        options.listen.clone()
    } else {
        let Some(config) = services.localHostConfig()? else {
            return Ok(());
        };
        let capabilities = services.listenerCapabilities()?;
        let mut transports = Vec::new();
        for transport in config.transports {
            if capabilities.transports.contains(&transport) && !transports.contains(&transport) {
                transports.push(transport);
            }
        }
        transports
    };
    if transports.is_empty() {
        return Ok(());
    }
    match services.startListening(transports).await {
        Ok(()) => Ok(()),
        Err(error) if explicit => Err(error),
        Err(error) => {
            AppLogger::w(
                "DaemonListener",
                &format!("automatic listener start failed: {error}"),
            );
            Ok(())
        }
    }
}

/// Starts a detached Core owner and waits for its ready status.
async fn start_background(args: &[String]) -> Result<(), String> {
    RunOptions::parse(args)?;
    let paths = DaemonPaths::for_active_identity()?;
    if let Some(status) = query_status_at(&paths).await? {
        if status.mode == "tui" {
            return Err(format!(
                "the TUI owns identity '{}' (pid {}); quit it before starting the daemon",
                paths.identity_name, status.pid
            ));
        }
        println!(
            "daemon already running (pid {}, state {})",
            status.pid, status.state
        );
        return Ok(());
    }
    if owner_lock_held(&paths) {
        return Err(format!(
            "a Core owner is already starting for identity '{}'; retry shortly",
            paths.identity_name
        ));
    }
    spawn_detached(args, &paths)?;
    let started = Instant::now();
    loop {
        if let Some(status) = query_status_at(&paths).await? {
            if status.state == "ready" {
                print_status_human(&status);
                return Ok(());
            }
        }
        if started.elapsed() >= DAEMON_READY_TIMEOUT {
            return Err(format!(
                "daemon did not become ready within {}s; see {}",
                DAEMON_READY_TIMEOUT.as_secs(),
                paths.output_log_path.display()
            ));
        }
        if !paths.status_path.exists() && started.elapsed() > Duration::from_secs(5) {
            return Err(format!(
                "daemon exited during startup; see {}",
                paths.output_log_path.display()
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Keeps the detached daemon output bounded by dropping its oldest records
/// before the new daemon reopens the file. Best effort: a failed trim must
/// never block daemon startup.
fn trim_daemon_output(path: &Path) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if !metadata.is_file() || metadata.len() <= DAEMON_OUT_MAX_BYTES {
        return;
    }
    let Ok(content) = fs::read(path) else {
        return;
    };
    // Raw process output may not be UTF-8: cut on byte boundaries only,
    // resuming at the next line start so every kept record stays whole.
    let mut cut = content.len().saturating_sub(DAEMON_OUT_KEEP_BYTES);
    while cut < content.len() && (content[cut] & 0xC0) == 0x80 {
        cut += 1;
    }
    while cut < content.len() && content[cut] != b'\n' {
        cut += 1;
    }
    let kept = if cut < content.len() {
        &content[cut + 1..]
    } else {
        &content[content.len()..]
    };
    let mut trimmed = b"daemon output trimmed; oldest records evicted\n".to_vec();
    trimmed.extend_from_slice(kept);
    let _ = fs::write(path, &trimmed);
}

/// Redirects daemon output and delegates process creation to the host.
fn spawn_detached(args: &[String], paths: &DaemonPaths) -> Result<u32, String> {
    if let Some(parent) = paths.output_log_path.parent() {
        ensure_private_dir(parent)?;
    }
    trim_daemon_output(&paths.output_log_path);
    let executable =
        std::env::current_exe().map_err(|error| format!("cannot resolve operit2: {error}"))?;
    let mut options = DaemonHost::private_open_options();
    options.create(true).append(true);
    let log = options.open(&paths.output_log_path).map_err(|error| {
        format!(
            "cannot open daemon output {}: {error}",
            paths.output_log_path.display()
        )
    })?;
    let stderr = log
        .try_clone()
        .map_err(|error| format!("cannot clone daemon output handle: {error}"))?;
    let mut command = Command::new(executable);
    command
        .arg("daemon")
        .arg("run")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr));
    let child = DaemonHost::spawn_detached(&mut command)
        .map_err(|error| format!("cannot start daemon: {error}"))?;
    Ok(child.id())
}

/// Prints the current identity owner status.
async fn status_command(json: bool) -> Result<(), String> {
    let paths = DaemonPaths::for_active_identity()?;
    let status = query_status_at(&paths).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "running": status.is_some(),
                "status": status,
            }))
            .map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    match status {
        Some(status) => print_status_human(&status),
        None => {
            if owner_lock_held(&paths) {
                if let Ok(status) = read_status(&paths.status_path) {
                    println!(
                        "daemon: starting (pid {}, mode {}, state {})",
                        status.pid, status.mode, status.state
                    );
                    println!("socket: {} (not ready)", status.socket);
                    return Ok(());
                }
            }
            println!("daemon: not running");
            if let Ok(stale) = read_status(&paths.status_path) {
                println!(
                    "stale status: pid {} ({}), socket {}",
                    stale.pid, stale.state, stale.socket
                );
            }
        }
    }
    Ok(())
}

/// Requests orderly shutdown and waits for the owner lock to release.
async fn stop_command(json: bool) -> Result<(), String> {
    let paths = DaemonPaths::for_active_identity()?;
    let Some(status) = query_status_at(&paths).await? else {
        if json {
            println!("{}", serde_json::json!({"running": false}));
        } else {
            println!("daemon: not running");
        }
        return Ok(());
    };
    if status.mode != "daemon" {
        return Err("the TUI owns this identity; quit it from the TUI".to_string());
    }
    let response = request_at_with_timeout(
        &paths.socket_path,
        &ControlRequest {
            op: "shutdown".to_string(),
            argv: Vec::new(),
        },
        Duration::from_secs(10),
    )
    .await?;
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| "shutdown request failed".to_string()));
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    while owner_lock_held(&paths) {
        if Instant::now() >= deadline {
            return Err(
                "shutdown was requested but the daemon is still running; check its log".to_string(),
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if json {
        println!(
            "{}",
            serde_json::json!({"running": false, "pid": status.pid})
        );
    } else {
        println!("daemon stopped (pid {})", status.pid);
    }
    Ok(())
}

/// Prints the human-readable owner diagnostics.
fn print_status_human(status: &DaemonStatus) {
    let label = if status.mode == "tui" {
        "owner"
    } else {
        "daemon"
    };
    println!(
        "{label}: running (pid {}, mode {}, state {})",
        status.pid, status.mode, status.state
    );
    println!(
        "identity: {} ({})",
        status.identity_name, status.identity_id
    );
    println!("socket: {}", status.socket);
    println!("log: {}", status.log_path);
    if status.mode == "daemon" {
        println!("stdout: {}", status.output_log);
    }
    if status.commands > 0 {
        println!("routed commands: {}", status.commands);
    }
    println!("version: {}", status.version);
}

/// Creates a daemon directory using its host's permission model.
fn ensure_private_dir(path: &Path) -> Result<(), String> {
    DaemonHost::ensure_private_dir(path).map_err(|error| {
        format!(
            "cannot create private directory {}: {error}",
            path.display()
        )
    })
}

/// Atomically publishes the complete status file with host permissions.
fn write_status(path: &Path, status: &DaemonStatus) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(status).map_err(|error| error.to_string())?;
    let mut options = DaemonHost::private_open_options();
    options.write(true).create(true).truncate(true);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync {}: {error}", temporary.display()))?;
    drop(file);
    fs::rename(&temporary, path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))
}

/// Reads one persisted owner status without altering it.
fn read_status(path: &Path) -> Result<DaemonStatus, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

/// Reads the wall-clock timestamp used by owner diagnostics.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that interactive and local-only commands bypass owner routing.
    #[test]
    fn interactive_commands_stay_in_the_invoking_process() {
        assert!(route_supported(&["package".into(), "list".into()]));
        assert!(route_supported(&["chat".into(), "list".into()]));
        assert!(!route_supported(&["chat".into(), "shell".into()]));
        assert!(!route_supported(&[
            "model".into(),
            "codex-login".into(),
            "device".into()
        ]));
        assert!(!route_supported(&[
            "link".into(),
            "space".into(),
            "show".into()
        ]));
        assert!(!route_supported(&["version".into()]));
        assert!(!route_supported(&[]));
    }

    /// Checks that ownership is exclusive and released when its guard drops.
    #[test]
    fn owner_lock_is_exclusive_per_identity() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "operit-daemon-test-{}-{unique}",
            std::process::id()
        ));
        let identity = CliIdentity {
            id: "test".to_string(),
            name: "Test".to_string(),
            createdAt: 0,
        };
        let paths = DaemonPaths::at(&root, &identity);
        let first = OwnerGuard::acquire_at(paths.clone(), "test").expect("first owner");
        assert!(OwnerGuard::acquire_at(paths.clone(), "test").is_err());
        drop(first);
        let third = OwnerGuard::acquire_at(paths, "test").expect("lock must be released");
        drop(third);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn daemon_output_is_trimmed_to_newest_records() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "operit-daemon-trim-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("daemon.out");
        let line = format!("{}x\n", "y".repeat(4096));
        let records = (DAEMON_OUT_MAX_BYTES / 4096 as u64 + 64) as usize;
        let body: String = (0..records).map(|index| format!("record-{index} {line}")).collect();
        fs::write(&path, &body).unwrap();

        trim_daemon_output(&path);
        let trimmed = fs::read(&path).unwrap();
        assert!(
            (trimmed.len() as u64) <= DAEMON_OUT_MAX_BYTES,
            "trimmed output must stay under the cap"
        );
        assert!(trimmed.starts_with(b"daemon output trimmed; oldest records evicted\n"));
        let text = String::from_utf8_lossy(&trimmed);
        assert!(!text.contains("record-0 "), "oldest records must be evicted");
        assert!(
            text.contains(&format!("record-{} ", records - 1)),
            "newest records must survive"
        );

        let before = fs::read(&path).unwrap();
        trim_daemon_output(&path);
        assert_eq!(fs::read(&path).unwrap(), before, "must be a no-op under the cap");
        let _ = fs::remove_dir_all(root);
    }
}
