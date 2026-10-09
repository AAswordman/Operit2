use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, BufReader, Read};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use operit_host_api::{
    HostRuntimeTaskSchedulerHost, HttpHost, HttpRequestData, HttpResponseData, ManagedRuntimeHost,
    ManagedRuntimeProcess, ManagedRuntimeProgram, RuntimeProcessRequest,
};
use serde_json::{json, Value};
use url::Url;

use operit_host_api::HostManager::HostManager;

#[path = "MCPStreamableHttp.rs"]
mod streamable_http;

const REQUEST_TIMEOUT_MS: u64 = 180_000;
const SPAWN_TIMEOUT_MS: u64 = 180_000;

/// Shares one deadline across process launch and every initialization request.
#[derive(Clone)]
struct StartupDeadline {
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    expiresAt: u64,
}

impl StartupDeadline {
    /// Binds every phase to the owning runtime's clock, never to a target/global clock.
    fn new(
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        timeoutMs: u64,
    ) -> Result<Self, String> {
        let now = scheduler
            .monotonicTimeMillis()
            .map_err(|error| error.to_string())?;
        let expiresAt = now
            .checked_add(timeoutMs)
            .ok_or_else(|| "MCP deadline exceeds the Host clock range".to_string())?;
        Ok(Self {
            scheduler,
            expiresAt,
        })
    }

    fn remainingMs(&self) -> Result<u64, String> {
        self.remainingAt(
            self.scheduler
                .monotonicTimeMillis()
                .map_err(|error| error.to_string())?,
        )
    }

    fn remainingAt(&self, now: u64) -> Result<u64, String> {
        self.expiresAt
            .checked_sub(now)
            .filter(|remaining| *remaining > 0)
            .ok_or_else(|| "MCP deadline exceeded".to_string())
    }
}

#[derive(Clone, Debug, Default)]
pub struct MCPBridge;

#[derive(Clone, Debug, Default)]
pub struct ServiceInfo {
    pub name: String,
    pub active: bool,
    pub ready: bool,
    pub toolCount: usize,
    pub toolNames: Vec<String>,
}

#[derive(Clone, Debug)]
struct RegisteredService {
    name: String,
    serviceType: String,
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
    endpoint: Option<String>,
    connectionType: Option<String>,
    bearerToken: Option<String>,
    headers: BTreeMap<String, String>,
    description: String,
    env: BTreeMap<String, String>,
}

struct ActiveService {
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    process: Option<Box<dyn ManagedRuntimeProcess>>,
    remote: Option<RemoteMcpSession>,
    requestId: u64,
    tools: Vec<Value>,
    ready: bool,
    logs: String,
}

struct RemoteMcpSession {
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    httpHost: Arc<dyn HttpHost>,
    endpoint: String,
    connectionType: String,
    headers: BTreeMap<String, String>,
    sessionId: Option<String>,
    protocolVersion: String,
    sseEndpoint: Option<String>,
    sseReader: Option<BufReader<RemoteSseReader>>,
    sseStreamId: Option<String>,
}

/// Bridges the host's live byte callbacks to the incremental SSE parser.
enum RemoteSseMessage {
    Bytes(Vec<u8>),
    Closed(Result<(), String>),
}

struct RemoteSseReader {
    messages: Receiver<RemoteSseMessage>,
    pending: Vec<u8>,
    offset: usize,
    deadline: StartupDeadline,
}

impl Read for RemoteSseReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        loop {
            let remaining = Duration::from_millis(
                self.deadline
                    .remainingMs()
                    .map_err(|error| io::Error::new(io::ErrorKind::TimedOut, error))?,
            );
            if self.offset < self.pending.len() {
                let count = output.len().min(self.pending.len() - self.offset);
                output[..count].copy_from_slice(&self.pending[self.offset..self.offset + count]);
                self.offset += count;
                return Ok(count);
            }
            match self.messages.recv_timeout(remaining) {
                Ok(RemoteSseMessage::Bytes(bytes)) => {
                    self.pending = bytes;
                    self.offset = 0;
                }
                Ok(RemoteSseMessage::Closed(result)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        result
                            .err()
                            .unwrap_or_else(|| "Remote MCP SSE stream closed".to_string()),
                    ));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Remote MCP SSE request timed out",
                    ));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Remote MCP SSE stream disconnected",
                    ));
                }
            }
        }
    }
}

impl Drop for RemoteMcpSession {
    fn drop(&mut self) {
        if let Some(streamId) = &self.sseStreamId {
            let _ = self.httpHost.closeHttpByteStream(streamId);
        }
    }
}

#[derive(Default)]
struct MCPBridgeState {
    services: BTreeMap<String, RegisteredService>,
    active: BTreeMap<String, ActiveService>,
    cachedTools: BTreeMap<String, Vec<Value>>,
    errors: BTreeMap<String, String>,
}

static STATE: OnceLock<Mutex<MCPBridgeState>> = OnceLock::new();

impl MCPBridge {
    #[allow(non_snake_case)]
    pub fn getInstance(_context: &HostManager) -> Self {
        Self
    }

    /// Registers a local MCP service process with command, arguments, environment, and working directory.
    #[allow(non_snake_case)]
    pub fn registerMcpService(
        &self,
        name: String,
        command: String,
        args: Vec<String>,
        description: Option<String>,
        env: BTreeMap<String, String>,
        cwd: Option<String>,
    ) -> Value {
        if name.trim().is_empty() || command.trim().is_empty() {
            return errorResponse("register", -32602, "Invalid local MCP service registration");
        }
        let mut state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        state.services.insert(
            name.clone(),
            RegisteredService {
                name: name.clone(),
                serviceType: "local".to_string(),
                command,
                args,
                cwd,
                endpoint: None,
                connectionType: None,
                bearerToken: None,
                headers: BTreeMap::new(),
                description: description.unwrap_or_else(|| format!("MCP Service: {name}")),
                env,
            },
        );
        successResponse("register", json!({ "name": name }))
    }

    /// Registers a remote MCP service endpoint with optional authentication headers.
    #[allow(non_snake_case)]
    pub fn registerRemoteMcpService(
        &self,
        name: String,
        endpoint: String,
        connectionType: Option<String>,
        description: Option<String>,
        bearerToken: Option<String>,
        headers: BTreeMap<String, String>,
    ) -> Value {
        if name.trim().is_empty() || endpoint.trim().is_empty() {
            return errorResponse(
                "register",
                -32602,
                "Invalid remote MCP service registration",
            );
        }
        let mut state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        state.services.insert(
            name.clone(),
            RegisteredService {
                name: name.clone(),
                serviceType: "remote".to_string(),
                command: String::new(),
                args: Vec::new(),
                cwd: None,
                endpoint: Some(endpoint),
                connectionType,
                bearerToken,
                headers,
                description: description.unwrap_or_else(|| format!("Remote MCP Service: {name}")),
                env: BTreeMap::new(),
            },
        );
        successResponse("register", json!({ "name": name }))
    }

    /// Unregisters an MCP service and stops its active process or session.
    #[allow(non_snake_case)]
    pub fn unregisterMcpService(&self, name: &str) -> Value {
        let mut state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        state.services.remove(name);
        if let Some(active) = state.active.remove(name) {
            if let Some(process) = active.process {
                let _ = process.kill();
            }
        }
        state.cachedTools.remove(name);
        state.errors.remove(name);
        successResponse("unregister", json!({ "name": name }))
    }

    /// Lists registered MCP services with active state and discovered tools.
    #[allow(non_snake_case)]
    pub fn listMcpServices(&self, serviceName: Option<&str>) -> Value {
        let state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        let mut services = Vec::new();
        for (name, registered) in &state.services {
            if serviceName.map(|target| target != name).unwrap_or(false) {
                continue;
            }
            let active = state.active.get(name);
            let cachedTools = state.cachedTools.get(name);
            let tools = active
                .map(|service| service.tools.clone())
                .or_else(|| cachedTools.cloned())
                .unwrap_or_default();
            services.push(json!({
                "name": name,
                "active": active.is_some(),
                "ready": active.map(|service| service.ready).unwrap_or(false),
                "toolCount": tools.len(),
                "tools": tools,
                "description": registered.description,
                "type": registered.serviceType,
            }));
        }
        successResponse("list", json!({ "services": services }))
    }

    /// Returns compact runtime information for one MCP service.
    #[allow(non_snake_case)]
    pub fn getServiceInfo(&self, serviceName: &str) -> Option<ServiceInfo> {
        let state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        let registered = state.services.get(serviceName)?;
        let active = state.active.get(serviceName);
        let tools = active
            .map(|service| service.tools.clone())
            .or_else(|| state.cachedTools.get(serviceName).cloned())
            .unwrap_or_default();
        let toolNames = tools
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str).map(str::to_string))
            .collect::<Vec<_>>();
        Some(ServiceInfo {
            name: registered.name.clone(),
            active: active.is_some(),
            ready: active.map(|service| service.ready).unwrap_or(false),
            toolCount: toolNames.len(),
            toolNames,
        })
    }

    /// Starts a registered MCP service and initializes its tool list.
    #[allow(non_snake_case)]
    pub fn spawnMcpService(
        &self,
        context: &HostManager,
        name: &str,
        timeoutMs: Option<u64>,
    ) -> Value {
        let registered = {
            let state = bridgeState().lock().expect("mcp bridge mutex poisoned");
            if state
                .active
                .get(name)
                .map(|service| service.ready)
                .unwrap_or(false)
            {
                let toolCount = state
                    .active
                    .get(name)
                    .map(|service| service.tools.len())
                    .unwrap_or(0);
                return successResponse(
                    "spawn",
                    json!({ "status": "started", "name": name, "toolCount": toolCount, "ready": true }),
                );
            }
            match state.services.get(name).cloned() {
                Some(service) => service,
                None => return errorResponse("spawn", -32602, "Service is not registered"),
            }
        };

        let Some(scheduler) = context.hostRuntimeTaskSchedulerHost.clone() else {
            return errorResponse(
                "spawn",
                -32603,
                "Runtime task scheduler host is not configured",
            );
        };
        let deadline =
            match StartupDeadline::new(scheduler.clone(), timeoutMs.unwrap_or(SPAWN_TIMEOUT_MS)) {
                Ok(deadline) => deadline,
                Err(message) => return errorResponse("spawn", -32603, &message),
            };
        if let Err(message) = deadline.remainingMs() {
            return errorResponse("spawn", -32603, &message);
        }
        let startResult = if registered.serviceType == "remote" {
            let Some(host) = context.httpHost.as_ref() else {
                return errorResponse("spawn", -32603, "HTTP host is not configured");
            };
            startRemoteServiceSession(host.clone(), &registered, &deadline)
        } else {
            let Some(host) = context.managedRuntimeHost.as_ref() else {
                return errorResponse("spawn", -32603, "Managed runtime host is not configured");
            };
            startLocalServiceProcess(host.as_ref(), &registered, scheduler)
        };
        let mut active = match startResult {
            Ok(value) => value,
            Err(message) => {
                bridgeState()
                    .lock()
                    .expect("mcp bridge mutex poisoned")
                    .errors
                    .insert(name.to_string(), message.clone());
                return errorResponse("spawn", -32603, &message);
            }
        };

        let initializeResult = if active.process.is_some() {
            initializeService(&mut active, &deadline)
        } else {
            Ok(())
        };
        match initializeResult {
            Ok(()) => {
                let toolCount = active.tools.len();
                bridgeState()
                    .lock()
                    .expect("mcp bridge mutex poisoned")
                    .active
                    .insert(name.to_string(), active);
                successResponse(
                    "spawn",
                    json!({ "status": "started", "name": name, "toolCount": toolCount, "ready": true }),
                )
            }
            Err(message) => {
                if let Some(process) = active.process {
                    let _ = process.kill();
                }
                bridgeState()
                    .lock()
                    .expect("mcp bridge mutex poisoned")
                    .errors
                    .insert(name.to_string(), message.clone());
                errorResponse("spawn", -32603, &message)
            }
        }
    }

    /// Stops an active MCP service without removing its registration.
    #[allow(non_snake_case)]
    pub fn unspawnMcpService(&self, name: &str) -> Value {
        let mut state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        if let Some(active) = state.active.remove(name) {
            if let Some(process) = active.process {
                let _ = process.kill();
            }
        }
        successResponse("unspawn", json!({ "name": name }))
    }

    /// Stores a tool list for a service when it is not currently active.
    #[allow(non_snake_case)]
    pub fn cacheTools(&self, serviceName: String, tools: Vec<Value>) -> Value {
        bridgeState()
            .lock()
            .expect("mcp bridge mutex poisoned")
            .cachedTools
            .insert(serviceName.clone(), tools);
        successResponse("cachetools", json!({ "name": serviceName }))
    }

    /// Lists tools from an active service or its cached tool metadata.
    #[allow(non_snake_case)]
    pub fn listTools(&self, serviceName: &str) -> Value {
        let state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        let tools = state
            .active
            .get(serviceName)
            .map(|service| service.tools.clone())
            .or_else(|| state.cachedTools.get(serviceName).cloned())
            .unwrap_or_default();
        successResponse("listtools", json!({ "tools": tools }))
    }

    /// Calls one tool on an active local or remote MCP service.
    #[allow(non_snake_case)]
    pub fn callTool(&self, serviceName: &str, method: &str, params: Value) -> Value {
        let mut state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        let Some(active) = state.active.get_mut(serviceName) else {
            return errorResponse("toolcall", -32603, "MCP service is not active");
        };
        let result = if active.remote.is_some() {
            callRemoteMcpTool(active, method, params, REQUEST_TIMEOUT_MS)
        } else {
            callMcpTool(active, method, params, REQUEST_TIMEOUT_MS)
        };
        match result {
            Ok(result) => successResponse("toolcall", result),
            Err(message) => errorResponse("toolcall", -32603, &message),
        }
    }

    /// Returns logs or the latest startup error for an MCP service.
    #[allow(non_snake_case)]
    pub fn getServiceLogs(&self, serviceName: &str) -> Value {
        let state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        let logs = state
            .active
            .get(serviceName)
            .map(|service| service.logs.clone())
            .or_else(|| state.errors.get(serviceName).cloned())
            .unwrap_or_default();
        successResponse("logs", json!({ "name": serviceName, "logs": logs }))
    }

    /// Stops all active MCP services and clears bridge registrations and cached tools.
    #[allow(non_snake_case)]
    pub fn resetBridge(&self) -> Value {
        let mut state = bridgeState().lock().expect("mcp bridge mutex poisoned");
        for (_, active) in std::mem::take(&mut state.active) {
            if let Some(process) = active.process {
                let _ = process.kill();
            }
        }
        state.services.clear();
        state.cachedTools.clear();
        state.errors.clear();
        successResponse("reset", json!({ "status": "reset" }))
    }
}

#[allow(non_snake_case)]
fn bridgeState() -> &'static Mutex<MCPBridgeState> {
    STATE.get_or_init(|| Mutex::new(MCPBridgeState::default()))
}

#[allow(non_snake_case)]
fn startLocalServiceProcess(
    host: &dyn ManagedRuntimeHost,
    service: &RegisteredService,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
) -> Result<ActiveService, String> {
    let runtime = resolveMcpRuntimeCommand(&service.command, &service.args)?;
    let process = host
        .startRuntimeProcess(RuntimeProcessRequest {
            program: runtime.program,
            executablePath: runtime.executablePath,
            args: runtime.args,
            cwd: service.cwd.as_deref().map(expandPath),
            env: buildRuntimeEnv(service),
        })
        .map_err(|error| error.to_string())?;
    Ok(ActiveService {
        scheduler,
        process: Some(process),
        remote: None,
        requestId: 0,
        tools: Vec::new(),
        ready: false,
        logs: String::new(),
    })
}

struct ResolvedRuntimeCommand {
    program: ManagedRuntimeProgram,
    executablePath: Option<String>,
    args: Vec<String>,
}

#[allow(non_snake_case)]
fn resolveMcpRuntimeCommand(
    command: &str,
    args: &[String],
) -> Result<ResolvedRuntimeCommand, String> {
    let commandName = command
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match commandName.as_str() {
        "node" | "node.exe" => Ok(ResolvedRuntimeCommand {
            program: ManagedRuntimeProgram::Node,
            executablePath: pathOverride(command, &["node", "node.exe"]),
            args: args.to_vec(),
        }),
        "python" | "python.exe" | "python3" => Ok(ResolvedRuntimeCommand {
            program: ManagedRuntimeProgram::Python,
            executablePath: pathOverride(command, &["python", "python.exe", "python3"]),
            args: args.to_vec(),
        }),
        "pnpm" | "pnpm.cmd" => Ok(ResolvedRuntimeCommand {
            program: ManagedRuntimeProgram::Pnpm,
            executablePath: pathOverride(command, &["pnpm", "pnpm.cmd"]),
            args: args.to_vec(),
        }),
        "npx" | "npx.cmd" => {
            let filteredArgs = args
                .iter()
                .filter(|arg| arg.as_str() != "-y" && arg.as_str() != "--yes")
                .cloned()
                .collect::<Vec<_>>();
            let mut runtimeArgs = vec!["dlx".to_string()];
            runtimeArgs.extend(filteredArgs);
            Ok(ResolvedRuntimeCommand {
                program: ManagedRuntimeProgram::Pnpm,
                executablePath: None,
                args: runtimeArgs,
            })
        }
        "uv" | "uv.exe" => Ok(ResolvedRuntimeCommand {
            program: ManagedRuntimeProgram::Uv,
            executablePath: pathOverride(command, &["uv", "uv.exe"]),
            args: args.to_vec(),
        }),
        "uvx" | "uvx.exe" => {
            let mut runtimeArgs = vec!["tool".to_string(), "run".to_string()];
            runtimeArgs.extend(args.iter().cloned());
            Ok(ResolvedRuntimeCommand {
                program: ManagedRuntimeProgram::Uv,
                executablePath: None,
                args: runtimeArgs,
            })
        }
        _ => Err(format!(
            "Unsupported MCP command '{command}'. Managed runtime supports node, python, uv, pnpm, npx, and uvx."
        )),
    }
}

#[allow(non_snake_case)]
fn pathOverride(command: &str, wellKnownNames: &[&str]) -> Option<String> {
    let expanded = expandPath(command);
    let trimmed = expanded.trim();
    if wellKnownNames
        .iter()
        .any(|name| trimmed.eq_ignore_ascii_case(name))
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
fn buildRuntimeEnv(service: &RegisteredService) -> BTreeMap<String, String> {
    let mut env = service.env.clone();
    if let Some(cwd) = &service.cwd {
        let cwd = expandPath(cwd);
        env.entry("npm_config_cache".to_string())
            .or_insert_with(|| format!("{cwd}/.npm-cache"));
    }
    env.entry("npm_config_prefer_offline".to_string())
        .or_insert_with(|| "true".to_string());
    env.entry("UV_LINK_MODE".to_string())
        .or_insert_with(|| "copy".to_string());
    #[cfg(target_os = "linux")]
    {
        env.entry("NODE_OPTIONS".to_string())
            .or_insert_with(|| "--openssl-legacy-provider".to_string());
    }
    env
}

#[allow(non_snake_case)]
fn expandPath(filePath: &str) -> String {
    let trimmed = filePath.trim();
    if trimmed == "~" {
        return homeDir()
            .expect("managed runtime host home directory must be available")
            .to_string_lossy()
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return homeDir()
            .expect("managed runtime host home directory must be available")
            .join(rest)
            .to_string_lossy()
            .to_string();
    }
    trimmed.to_string()
}

#[allow(non_snake_case)]
fn homeDir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Initializes a remote session within the shared startup deadline.
#[allow(non_snake_case)]
fn startRemoteServiceSession(
    httpHost: Arc<dyn HttpHost>,
    service: &RegisteredService,
    deadline: &StartupDeadline,
) -> Result<ActiveService, String> {
    let connectionType = service.connectionType.as_deref().unwrap_or("httpStream");
    let endpoint = service
        .endpoint
        .clone()
        .ok_or_else(|| "Remote MCP service endpoint is empty".to_string())?;
    let mut headers = service.headers.clone();
    if let Some(token) = &service.bearerToken {
        headers.insert("Authorization".to_string(), format!("Bearer {token}"));
    }
    let mut active = ActiveService {
        scheduler: deadline.scheduler.clone(),
        process: None,
        remote: Some(RemoteMcpSession {
            scheduler: deadline.scheduler.clone(),
            httpHost,
            endpoint,
            connectionType: connectionType.to_string(),
            headers,
            sessionId: None,
            protocolVersion: "2024-11-05".to_string(),
            sseEndpoint: None,
            sseReader: None,
            sseStreamId: None,
        }),
        requestId: 0,
        tools: Vec::new(),
        ready: false,
        logs: String::new(),
    };

    if connectionType.eq_ignore_ascii_case("sse") {
        connectRemoteSse(
            active
                .remote
                .as_mut()
                .ok_or_else(|| "Remote MCP session is not attached".to_string())?,
            deadline.remainingMs()?,
        )?;
    }
    initializeRemoteService(&mut active, deadline)?;
    Ok(active)
}

#[allow(non_snake_case)]
fn connectRemoteSse(session: &mut RemoteMcpSession, timeoutMs: u64) -> Result<(), String> {
    if session.httpHost.responseDelivery() == operit_host_api::HttpResponseDelivery::Buffered {
        return Err(
            "HTTP Host only supports finite responses, not live MCP SSE sessions".to_string(),
        );
    }

    let deadline = StartupDeadline::new(session.scheduler.clone(), timeoutMs)?;
    let streamId = format!("mcp-sse-{}", uuid::Uuid::new_v4());
    let (sender, messages) = mpsc::channel();
    let chunkSender = sender.clone();
    let mut request = remoteHttpRequest(
        "GET",
        &session.endpoint,
        buildRemoteHeaders(session, false)?,
        Vec::new(),
        timeoutMs,
    );
    // The startup budget bounds endpoint discovery, not the lifetime of this stream.
    // Each protocol read has its own deadline and Drop cancels the host connection.
    request.readTimeoutSeconds = 0;
    session
        .httpHost
        .openHttpByteStream(
            streamId.clone(),
            request,
            Arc::new(|| {}),
            Arc::new(move |bytes| {
                let _ = chunkSender.send(RemoteSseMessage::Bytes(bytes));
            }),
            Arc::new(move |result| {
                let _ = sender.send(RemoteSseMessage::Closed(result));
            }),
        )
        .map_err(|error| error.to_string())?;
    // Store the stream before reading so any failed handshake cancels it on drop.
    session.sseStreamId = Some(streamId);
    let mut reader = BufReader::new(RemoteSseReader {
        messages,
        pending: Vec::new(),
        offset: 0,
        deadline: deadline.clone(),
    });
    loop {
        deadline
            .remainingMs()
            .map_err(|error| format!("Remote MCP SSE endpoint event timed out: {error}"))?;
        let Some((eventName, data)) = readSseEvent(&mut reader)? else {
            continue;
        };
        if eventName == "endpoint" {
            let base = Url::parse(&session.endpoint).map_err(|error| error.to_string())?;
            let endpoint = base.join(data.trim()).map_err(|error| error.to_string())?;
            if endpoint.origin() != base.origin() {
                return Err(format!(
                    "Endpoint origin does not match connection origin: {}",
                    endpoint.origin().ascii_serialization()
                ));
            }
            session.sseEndpoint = Some(endpoint.to_string());
            session.sseReader = Some(reader);
            return Ok(());
        }
    }
}

/// Initializes a remote server without resetting the startup budget.
#[allow(non_snake_case)]
fn initializeRemoteService(
    active: &mut ActiveService,
    deadline: &StartupDeadline,
) -> Result<(), String> {
    let initializeId = nextRequestId(active);
    let initializeResponse = sendRemoteJsonRpc(
        active
            .remote
            .as_mut()
            .ok_or_else(|| "Remote MCP session is not attached".to_string())?,
        json!({
            "jsonrpc": "2.0",
            "id": initializeId,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "operit2", "version": "1.0.0" }
            }
        }),
        Some(initializeId),
        deadline.remainingMs()?,
    )?
    .ok_or_else(|| "Remote MCP initialize returned an empty response".to_string())?;
    if initializeResponse.get("error").is_some() {
        return Err(format!("MCP initialize failed: {initializeResponse}"));
    }

    if let Some(version) = initializeResponse
        .get("result")
        .and_then(|result| result.get("protocolVersion"))
        .and_then(Value::as_str)
    {
        if !version.trim().is_empty() {
            active
                .remote
                .as_mut()
                .expect("remote session attached")
                .protocolVersion = version.to_string();
        }
    }

    let _ = sendRemoteJsonRpc(
        active
            .remote
            .as_mut()
            .ok_or_else(|| "Remote MCP session is not attached".to_string())?,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        None,
        deadline.remainingMs()?,
    )?;

    let listId = nextRequestId(active);
    let listResponse = sendRemoteJsonRpc(
        active
            .remote
            .as_mut()
            .ok_or_else(|| "Remote MCP session is not attached".to_string())?,
        json!({
            "jsonrpc": "2.0",
            "id": listId,
            "method": "tools/list",
            "params": {}
        }),
        Some(listId),
        deadline.remainingMs()?,
    )?
    .ok_or_else(|| "Remote MCP tools/list returned an empty response".to_string())?;
    if listResponse.get("error").is_some() {
        return Err(format!("MCP tools/list failed: {listResponse}"));
    }
    active.tools = listResponse
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    deadline.remainingMs()?;
    active.ready = true;
    Ok(())
}

/// Initializes a local server without resetting the startup budget.
#[allow(non_snake_case)]
fn initializeService(active: &mut ActiveService, deadline: &StartupDeadline) -> Result<(), String> {
    deadline.remainingMs()?;
    let initializeId = nextRequestId(active);
    let process = active
        .process
        .as_ref()
        .ok_or_else(|| "Local MCP process is not attached".to_string())?;
    process
        .writeLine(
            &json!({
                "jsonrpc": "2.0",
                "id": initializeId,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "operit2", "version": "1.0.0" }
                }
            })
            .to_string(),
        )
        .map_err(|error| error.to_string())?;
    let initializeResponse = readJsonResponse(active, initializeId, deadline.remainingMs()?)?;
    if initializeResponse.get("error").is_some() {
        return Err(format!("MCP initialize failed: {initializeResponse}"));
    }
    deadline.remainingMs()?;
    let listId = nextRequestId(active);
    let postInitializeMessages = vec![
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        })
        .to_string(),
        json!({
            "jsonrpc": "2.0",
            "id": listId,
            "method": "tools/list",
            "params": {}
        })
        .to_string(),
    ];
    let process = active
        .process
        .as_ref()
        .ok_or_else(|| "Local MCP process is not attached".to_string())?;
    process
        .writeLines(&postInitializeMessages)
        .map_err(|error| error.to_string())?;
    let listResponse = readJsonResponse(active, listId, deadline.remainingMs()?)?;
    if listResponse.get("error").is_some() {
        return Err(format!("MCP tools/list failed: {listResponse}"));
    }
    active.tools = listResponse
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    deadline.remainingMs()?;
    active.ready = true;
    Ok(())
}

#[allow(non_snake_case)]
fn callMcpTool(
    active: &mut ActiveService,
    method: &str,
    params: Value,
    timeoutMs: u64,
) -> Result<Value, String> {
    let id = nextRequestId(active);
    let process = active
        .process
        .as_ref()
        .ok_or_else(|| "Local MCP process is not attached".to_string())?;
    process
        .writeLine(
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {
                    "name": method,
                    "arguments": params
                }
            })
            .to_string(),
        )
        .map_err(|error| error.to_string())?;
    let response = readJsonResponse(active, id, timeoutMs)?;
    if let Some(error) = response.get("error") {
        return Err(format!("{error}"));
    }
    let result = response.get("result").cloned().unwrap_or_else(|| json!({}));
    if result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(format!("{result}"));
    }
    Ok(result)
}

#[allow(non_snake_case)]
fn callRemoteMcpTool(
    active: &mut ActiveService,
    method: &str,
    params: Value,
    timeoutMs: u64,
) -> Result<Value, String> {
    let id = nextRequestId(active);
    let response = sendRemoteJsonRpc(
        active
            .remote
            .as_mut()
            .ok_or_else(|| "Remote MCP session is not attached".to_string())?,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": method,
                "arguments": params
            }
        }),
        Some(id),
        timeoutMs,
    )?
    .ok_or_else(|| format!("Remote MCP tools/call returned an empty response for {method}"))?;
    if let Some(error) = response.get("error") {
        return Err(format!("{error}"));
    }
    let result = response.get("result").cloned().unwrap_or_else(|| json!({}));
    if result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(format!("{result}"));
    }
    Ok(result)
}

#[allow(non_snake_case)]
fn sendRemoteJsonRpc(
    session: &mut RemoteMcpSession,
    payload: Value,
    expectedId: Option<u64>,
    _timeoutMs: u64,
) -> Result<Option<Value>, String> {
    if session.connectionType.eq_ignore_ascii_case("sse") {
        return sendRemoteSseJsonRpc(session, payload, expectedId, _timeoutMs);
    }
    match session.httpHost.responseDelivery() {
        operit_host_api::HttpResponseDelivery::Incremental => {
            streamable_http::sendJsonRpc(session, payload, expectedId, _timeoutMs)
        }
        operit_host_api::HttpResponseDelivery::Buffered => {
            sendRemoteBufferedJsonRpc(session, payload, expectedId, _timeoutMs)
        }
    }
}

#[allow(non_snake_case)]
fn sendRemoteBufferedJsonRpc(
    session: &mut RemoteMcpSession,
    payload: Value,
    expectedId: Option<u64>,
    _timeoutMs: u64,
) -> Result<Option<Value>, String> {
    let deadline = StartupDeadline::new(session.scheduler.clone(), _timeoutMs)?;
    let body = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    let response = executeRemoteHttp(
        session,
        "POST",
        &session.endpoint,
        buildRemoteHeaders(session, true)?,
        body,
        _timeoutMs,
    )?;
    deadline
        .remainingMs()
        .map_err(|error| format!("Remote MCP HTTP request timed out: {error}"))?;
    rememberRemoteSessionId(session, &response.headers)?;
    let contentType = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.as_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let text = String::from_utf8(response.body).map_err(|error| error.to_string())?;
    if !isSuccess(response.statusCode) {
        return Err(format!(
            "Remote MCP HTTP request failed with status {}: {}",
            response.statusCode,
            text.trim()
        ));
    }
    let Some(expectedId) = expectedId else {
        return Ok(None);
    };
    let trimmed = text.trim();
    let parsed = if contentType.contains("text/event-stream")
        || trimmed.lines().any(|line| line.starts_with("data:"))
    {
        parseSseJsonResponse(trimmed, Some(expectedId))?
    } else if trimmed.is_empty() {
        None
    } else {
        Some(serde_json::from_str::<Value>(trimmed).map_err(|error| error.to_string())?)
    };
    match parsed {
        Some(value) if value.get("id").and_then(Value::as_u64) == Some(expectedId) => {
            Ok(Some(value))
        }
        _ => Err(format!(
            "Remote MCP HTTP response did not contain JSON-RPC response {expectedId}"
        )),
    }
}

#[allow(non_snake_case)]
fn sendRemoteSseJsonRpc(
    session: &mut RemoteMcpSession,
    payload: Value,
    expectedId: Option<u64>,
    timeoutMs: u64,
) -> Result<Option<Value>, String> {
    let deadline = StartupDeadline::new(session.scheduler.clone(), timeoutMs)?;
    let endpoint = session
        .sseEndpoint
        .clone()
        .ok_or_else(|| "Remote MCP SSE endpoint is not connected".to_string())?;
    let body = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    let response = executeRemoteHttp(
        session,
        "POST",
        &endpoint,
        buildRemoteHeaders(session, true)?,
        body,
        timeoutMs,
    )?;
    let text = String::from_utf8(response.body).map_err(|error| error.to_string())?;
    if !isSuccess(response.statusCode) {
        return Err(format!(
            "Remote MCP SSE POST failed with status {}: {}",
            response.statusCode,
            text.trim()
        ));
    }
    let Some(expectedId) = expectedId else {
        return Ok(None);
    };
    let reader = session
        .sseReader
        .as_mut()
        .ok_or_else(|| "Remote MCP SSE reader is not connected".to_string())?;
    reader.get_mut().deadline = deadline.clone();
    loop {
        deadline
            .remainingMs()
            .map_err(|error| format!("Remote MCP SSE request {expectedId} timed out: {error}"))?;
        let Some((eventName, data)) = readSseEvent(reader)? else {
            continue;
        };
        if eventName != "message" && !eventName.is_empty() {
            continue;
        }
        let parsed =
            serde_json::from_str::<Value>(data.trim()).map_err(|error| error.to_string())?;
        if parsed.get("id").and_then(Value::as_u64) == Some(expectedId) {
            return Ok(Some(parsed));
        }
    }
}

#[allow(non_snake_case)]
fn buildRemoteHeaders(
    session: &RemoteMcpSession,
    jsonBody: bool,
) -> Result<Vec<(String, String)>, String> {
    let mut headers = Vec::new();
    if jsonBody {
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
        headers.push((
            "Accept".to_string(),
            "application/json, text/event-stream".to_string(),
        ));
    } else {
        headers.push(("Accept".to_string(), "text/event-stream".to_string()));
    }
    headers.push((
        "mcp-protocol-version".to_string(),
        session.protocolVersion.clone(),
    ));
    if let Some(sessionId) = &session.sessionId {
        headers.push(("mcp-session-id".to_string(), sessionId.to_string()));
    }
    for (name, value) in &session.headers {
        if name.trim().is_empty() {
            return Err("Invalid MCP remote header: empty name".to_string());
        }
        headers.push((name.clone(), value.clone()));
    }
    Ok(headers)
}

#[allow(non_snake_case)]
fn rememberRemoteSessionId(
    session: &mut RemoteMcpSession,
    headers: &[(String, String)],
) -> Result<(), String> {
    if let Some((_, sessionId)) = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("mcp-session-id"))
    {
        let trimmed = sessionId.trim();
        if !trimmed.is_empty() {
            session.sessionId = Some(trimmed.to_string());
        }
    }
    Ok(())
}

#[allow(non_snake_case)]
fn executeRemoteHttp(
    session: &RemoteMcpSession,
    method: &str,
    endpoint: &str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeoutMs: u64,
) -> Result<HttpResponseData, String> {
    session
        .httpHost
        .executeHttpRequest(remoteHttpRequest(
            method, endpoint, headers, body, timeoutMs,
        ))
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn remoteHttpRequest(
    method: &str,
    endpoint: &str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeoutMs: u64,
) -> HttpRequestData {
    let timeoutSeconds = timeoutMs.div_ceil(1000).max(1);
    HttpRequestData {
        url: endpoint.to_string(),
        method: method.to_string(),
        headers,
        body,
        formFields: Vec::new(),
        fileParts: Vec::new(),
        connectTimeoutSeconds: timeoutSeconds,
        readTimeoutSeconds: timeoutSeconds,
        followRedirects: true,
        ignoreSsl: false,
        proxyHost: String::new(),
        proxyPort: 0,
    }
}

#[allow(non_snake_case)]
fn isSuccess(statusCode: i32) -> bool {
    (200..300).contains(&statusCode)
}

#[allow(non_snake_case)]
fn readSseEvent<R: BufRead>(reader: &mut R) -> Result<Option<(String, String)>, String> {
    let mut eventName = String::new();
    let mut dataLines = Vec::new();
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(&['\r', '\n'][..]);
        if line.is_empty() {
            if dataLines.is_empty() {
                return Ok(None);
            }
            return Ok(Some((eventName, dataLines.join("\n"))));
        }
        if let Some(event) = line.strip_prefix("event:") {
            eventName = event.trim_start().to_string();
        } else if let Some(data) = line.strip_prefix("data:") {
            dataLines.push(data.trim_start().to_string());
        }
    }
}

#[allow(non_snake_case)]
fn parseSseJsonResponse(text: &str, expectedId: Option<u64>) -> Result<Option<Value>, String> {
    let mut eventPayloads = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            if !current.trim().is_empty() {
                eventPayloads.push(current.trim().to_string());
                current.clear();
            }
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(data.trim_start());
        }
    }
    if !current.trim().is_empty() {
        eventPayloads.push(current.trim().to_string());
    }

    for payload in &eventPayloads {
        if payload == "[DONE]" {
            continue;
        }
        let parsed = serde_json::from_str::<Value>(payload).map_err(|error| error.to_string())?;
        if expectedId
            .map(|id| parsed.get("id").and_then(Value::as_u64) == Some(id))
            .unwrap_or(true)
        {
            return Ok(Some(parsed));
        }
    }
    Ok(None)
}

#[allow(non_snake_case)]
fn nextRequestId(active: &mut ActiveService) -> u64 {
    active.requestId += 1;
    active.requestId
}

/// Reads a matching response and reports process exit without waiting for the deadline.
#[allow(non_snake_case)]
fn readJsonResponse(
    active: &mut ActiveService,
    targetId: u64,
    timeoutMs: u64,
) -> Result<Value, String> {
    let deadline = StartupDeadline::new(active.scheduler.clone(), timeoutMs)?;
    let mut seenIds = BTreeSet::new();
    loop {
        let remaining = deadline.remainingMs();
        if remaining.is_err() {
            let stderr = active
                .process
                .as_ref()
                .ok_or_else(|| "Local MCP process is not attached".to_string())?
                .drainStderr()
                .unwrap_or_default();
            active.logs.push_str(&stderr);
            return Err(format!(
                "MCP request {targetId} timed out: {}. {stderr}",
                remaining.unwrap_err()
            ));
        }
        let waitMs = remaining?.min(250);
        let line = active
            .process
            .as_ref()
            .ok_or_else(|| "Local MCP process is not attached".to_string())?
            .readStdoutLine(waitMs)
            .map_err(|error| error.to_string())?;
        let Some(line) = line else {
            let process = active
                .process
                .as_ref()
                .ok_or_else(|| "Local MCP process is not attached".to_string())?;
            if !process.isRunning().map_err(|error| error.to_string())? {
                let stderr = process.drainStderr().map_err(|error| error.to_string())?;
                active.logs.push_str(&stderr);
                active.ready = false;
                return Err(format!(
                    "MCP process exited before response {targetId}. {stderr}"
                ));
            }
            continue;
        };
        let parsed = match serde_json::from_str::<Value>(&line) {
            Ok(value) => value,
            Err(_) => {
                active.logs.push_str(&line);
                active.logs.push('\n');
                continue;
            }
        };
        let Some(id) = parsed.get("id").and_then(Value::as_u64) else {
            continue;
        };
        seenIds.insert(id);
        if id == targetId {
            let stderr = active
                .process
                .as_ref()
                .ok_or_else(|| "Local MCP process is not attached".to_string())?
                .drainStderr()
                .unwrap_or_default();
            if !stderr.is_empty() {
                active.logs.push_str(&stderr);
            }
            return Ok(parsed);
        }
    }
}

#[allow(non_snake_case)]
fn successResponse(id: &str, result: Value) -> Value {
    json!({
        "id": id,
        "success": true,
        "result": result,
    })
}

#[allow(non_snake_case)]
fn errorResponse(id: &str, code: i64, message: &str) -> Value {
    json!({
        "id": id,
        "success": false,
        "error": {
            "code": code,
            "message": message
        }
    })
}

#[cfg(test)]
mod startup_tests {
    use super::*;
    use crate::tools::mcp_runtime::plugins::MCPBridgeClient::MCPBridgeClient;
    use operit_host_api::{HostError, HostResult};
    use std::collections::VecDeque;

    struct ScriptedProcess {
        responses: Mutex<VecDeque<Option<String>>>,
        writes: Arc<Mutex<Vec<String>>>,
        running: bool,
    }

    impl ManagedRuntimeProcess for ScriptedProcess {
        /// Records one outgoing JSON-RPC message without starting a real process.
        fn writeLine(&self, line: &str) -> HostResult<()> {
            self.writes.lock().unwrap().push(line.to_string());
            Ok(())
        }

        /// Records the initialized notification and tool-discovery request in order.
        fn writeLines(&self, lines: &[String]) -> HostResult<()> {
            self.writes.lock().unwrap().extend_from_slice(lines);
            Ok(())
        }

        /// Consumes exactly one scripted read and rejects unexpected additional reads.
        fn readStdoutLine(&self, _timeoutMs: u64) -> HostResult<Option<String>> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| HostError::new("Unexpected MCP stdout read"))
        }

        /// Supplies deterministic diagnostics for process-exit tests.
        fn drainStderr(&self) -> HostResult<String> {
            Ok("scripted stderr".to_string())
        }

        /// Reports the process state selected by the test.
        fn isRunning(&self) -> HostResult<bool> {
            Ok(self.running)
        }

        /// Acknowledges cleanup of the in-memory process used by the test.
        fn kill(&self) -> HostResult<()> {
            Ok(())
        }
    }

    /// Wraps a scripted process with isolated MCP protocol state.
    fn activeProcess(process: ScriptedProcess) -> ActiveService {
        ActiveService {
            scheduler: testScheduler(),
            process: Some(Box::new(process)),
            remote: None,
            requestId: 0,
            tools: Vec::new(),
            ready: false,
            logs: String::new(),
        }
    }

    struct ScriptedClock {
        readings: Mutex<VecDeque<HostResult<u64>>>,
    }

    impl HostRuntimeTaskSchedulerHost for ScriptedClock {
        fn monotonicTimeMillis(&self) -> HostResult<u64> {
            self.readings
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected clock read")
        }
        fn scheduleHostRuntimeTask(
            &self,
            _: &str,
            _: operit_host_api::HostRuntimeTask,
        ) -> HostResult<()> {
            unreachable!()
        }
        fn scheduleHostRuntimeAsyncTask(
            &self,
            _: &str,
            _: operit_host_api::HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            unreachable!()
        }
        fn scheduleDelayedHostRuntimeTask(
            &self,
            _: &str,
            _: u64,
            _: operit_host_api::HostRuntimeTask,
        ) -> HostResult<()> {
            unreachable!()
        }
        fn waitForHostRuntimeTaskTurn(&self) -> operit_host_api::HostRuntimeTurnFuture {
            unreachable!()
        }
        fn waitForHostRuntimeDelay(&self, _: u64) -> operit_host_api::HostRuntimeTurnFuture {
            unreachable!()
        }
    }

    fn scriptedClock(readings: Vec<HostResult<u64>>) -> Arc<dyn HostRuntimeTaskSchedulerHost> {
        Arc::new(ScriptedClock {
            readings: Mutex::new(readings.into()),
        })
    }

    /// Proves that no process/global/target clock replaces the supplied Host clock.
    #[test]
    fn startupDeadlineUsesItsOwningHostClock() {
        let deadline = StartupDeadline::new(
            scriptedClock(vec![Ok(4000), Ok(4200), Ok(4999), Ok(5000)]),
            1000,
        )
        .unwrap();
        assert_eq!(deadline.remainingMs().unwrap(), 800);
        assert_eq!(deadline.remainingMs().unwrap(), 1);
        assert!(deadline.remainingMs().is_err());
    }

    #[test]
    fn clockFailuresAndOverflowAreNotReplacedByFallbacks() {
        assert!(StartupDeadline::new(
            scriptedClock(vec![Err(HostError::new("clock unavailable"))]),
            1000
        )
        .err()
        .unwrap()
        .contains("clock unavailable"));
        let deadline = StartupDeadline::new(
            scriptedClock(vec![Ok(10), Err(HostError::new("clock lost"))]),
            1000,
        )
        .unwrap();
        assert!(deadline.remainingMs().unwrap_err().contains("clock lost"));
        assert!(StartupDeadline::new(scriptedClock(vec![Ok(u64::MAX)]), 1).is_err());
    }

    #[test]
    fn spawnWithoutSchedulerFailsBeforeLaunchingAProcess() {
        let context = HostManager::default();
        let bridge = MCPBridge::getInstance(&context);
        let name = format!("test-no-clock-{}", uuid::Uuid::new_v4());
        bridge.registerMcpService(
            name.clone(),
            "node".into(),
            Vec::new(),
            None,
            BTreeMap::new(),
            None,
        );
        let response = bridge.spawnMcpService(&context, &name, Some(1000));
        bridge.unregisterMcpService(&name);
        assert_eq!(response["success"], false);
        assert_eq!(
            response["error"]["message"],
            "Runtime task scheduler host is not configured"
        );
    }

    /// Deducts elapsed time from one shared budget across handshake phases.
    #[test]
    fn startupBudgetIsNotRenewed() {
        let deadline = StartupDeadline {
            scheduler: testScheduler(),
            expiresAt: 1100,
        };
        assert_eq!(deadline.remainingAt(100).unwrap(), 1000);
        assert_eq!(deadline.remainingAt(800).unwrap(), 300);
        assert_eq!(deadline.remainingAt(1099).unwrap(), 1);
        assert!(deadline.remainingAt(1100).is_err());
        assert!(deadline.remainingAt(1101).is_err());
    }

    /// Rejects an exhausted launch budget before sending the initialize request.
    #[test]
    fn expiredStartupDoesNotSendInitialize() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let mut active = activeProcess(ScriptedProcess {
            responses: Mutex::new(VecDeque::new()),
            writes: writes.clone(),
            running: true,
        });
        assert!(initializeService(
            &mut active,
            &StartupDeadline {
                scheduler: testScheduler(),
                expiresAt: 0
            }
        )
        .is_err());
        assert!(writes.lock().unwrap().is_empty());
        assert!(!active.ready);
    }

    /// Preserves exit diagnostics and fails immediately after stdout closes.
    #[test]
    fn exitedProcessFailsOnFirstEmptyRead() {
        let mut active = activeProcess(ScriptedProcess {
            responses: Mutex::new(VecDeque::from([None])),
            writes: Arc::new(Mutex::new(Vec::new())),
            running: false,
        });
        active.ready = true;
        assert_eq!(
            readJsonResponse(&mut active, 7, 180_000).unwrap_err(),
            "MCP process exited before response 7. scripted stderr"
        );
        assert!(!active.ready);
        assert_eq!(active.logs, "scripted stderr");
    }

    /// Prevents a timeout-shaped server error from executing a side-effecting tool twice.
    #[test]
    fn timeoutErrorDoesNotReplayToolCall() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let serverError = json!({ "code": -32000, "message": "timeout after side effect" });
        let mut active = activeProcess(ScriptedProcess {
            responses: Mutex::new(VecDeque::from([Some(
                json!({
                    "jsonrpc": "2.0", "id": 1, "error": serverError.clone()
                })
                .to_string(),
            )])),
            writes: writes.clone(),
            running: true,
        });
        active.ready = true;
        let context = HostManager::default().withHostRuntimeTaskSchedulerHost(testScheduler());
        let bridge = MCPBridge::getInstance(&context);
        let name = format!("test-no-replay-{}", uuid::Uuid::new_v4());
        bridge.registerMcpService(
            name.clone(),
            "node".to_string(),
            Vec::new(),
            Some("test".to_string()),
            BTreeMap::new(),
            None,
        );
        bridgeState()
            .lock()
            .unwrap()
            .active
            .insert(name.clone(), active);
        let client = MCPBridgeClient::new(context, name.clone());
        let response = client.callTool("write_file", json!({"path": "test.txt"}));
        bridge.unregisterMcpService(&name);
        assert_eq!(response["success"], false);
        assert_eq!(response["error"]["message"], serverError.to_string());
        assert_eq!(writes.lock().unwrap().len(), 1);
    }
}

#[cfg(test)]
#[path = "MCPBridgeRemoteTests.rs"]
mod remote_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
fn testScheduler() -> Arc<dyn HostRuntimeTaskSchedulerHost> {
    Arc::new(operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new())
}
