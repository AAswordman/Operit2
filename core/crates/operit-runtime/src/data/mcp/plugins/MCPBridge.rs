use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Cursor};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use operit_host_api::{
    HttpHost, HttpRequestData, HttpResponseData, ManagedRuntimeHost, ManagedRuntimeProcess,
    ManagedRuntimeProgram, RuntimeProcessRequest,
};
use serde_json::{json, Value};
use url::Url;

use operit_context::OperitApplicationContext::OperitApplicationContext;

const REQUEST_TIMEOUT_MS: u64 = 180_000;
const SPAWN_TIMEOUT_MS: u64 = 180_000;

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
    process: Option<Box<dyn ManagedRuntimeProcess>>,
    remote: Option<RemoteMcpSession>,
    requestId: u64,
    tools: Vec<Value>,
    ready: bool,
    logs: String,
}

struct RemoteMcpSession {
    httpHost: Arc<dyn HttpHost>,
    endpoint: String,
    connectionType: String,
    headers: BTreeMap<String, String>,
    sessionId: Option<String>,
    sseEndpoint: Option<String>,
    sseReader: Option<BufReader<Cursor<Vec<u8>>>>,
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
    pub fn getInstance(_context: &OperitApplicationContext) -> Self {
        Self
    }

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

    #[allow(non_snake_case)]
    pub fn spawnMcpService(
        &self,
        context: &OperitApplicationContext,
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

        let startResult = if registered.serviceType == "remote" {
            let Some(host) = context.httpHost.as_ref() else {
                return errorResponse("spawn", -32603, "HTTP host is not configured");
            };
            startRemoteServiceSession(
                host.clone(),
                &registered,
                timeoutMs.unwrap_or(SPAWN_TIMEOUT_MS),
            )
        } else {
            let Some(host) = context.managedRuntimeHost.as_ref() else {
                return errorResponse("spawn", -32603, "Managed runtime host is not configured");
            };
            startLocalServiceProcess(host.as_ref(), &registered)
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

        let timeout = timeoutMs.unwrap_or(SPAWN_TIMEOUT_MS);
        let initializeResult = if active.process.is_some() {
            initializeService(&mut active, timeout)
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

    #[allow(non_snake_case)]
    pub fn cacheTools(&self, serviceName: String, tools: Vec<Value>) -> Value {
        bridgeState()
            .lock()
            .expect("mcp bridge mutex poisoned")
            .cachedTools
            .insert(serviceName.clone(), tools);
        successResponse("cachetools", json!({ "name": serviceName }))
    }

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

#[allow(non_snake_case)]
fn startRemoteServiceSession(
    httpHost: Arc<dyn HttpHost>,
    service: &RegisteredService,
    timeoutMs: u64,
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
        process: None,
        remote: Some(RemoteMcpSession {
            httpHost,
            endpoint,
            connectionType: connectionType.to_string(),
            headers,
            sessionId: None,
            sseEndpoint: None,
            sseReader: None,
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
            timeoutMs,
        )?;
    }
    initializeRemoteService(&mut active, timeoutMs)?;
    Ok(active)
}

#[allow(non_snake_case)]
fn connectRemoteSse(session: &mut RemoteMcpSession, timeoutMs: u64) -> Result<(), String> {
    let response = executeRemoteHttp(
        session,
        "GET",
        &session.endpoint,
        buildRemoteHeaders(session, false)?,
        Vec::new(),
        timeoutMs,
    )?;
    if !isSuccess(response.statusCode) {
        return Err(format!(
            "Remote MCP SSE connect failed with status {}",
            response.statusCode
        ));
    }
    rememberRemoteSessionId(session, &response.headers)?;
    let mut reader = BufReader::new(Cursor::new(response.body));
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    loop {
        if Instant::now() >= deadline {
            return Err("Remote MCP SSE endpoint event timed out".to_string());
        }
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

#[allow(non_snake_case)]
fn initializeRemoteService(active: &mut ActiveService, timeoutMs: u64) -> Result<(), String> {
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
        timeoutMs,
    )?
    .ok_or_else(|| "Remote MCP initialize returned an empty response".to_string())?;
    if initializeResponse.get("error").is_some() {
        return Err(format!("MCP initialize failed: {initializeResponse}"));
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
        timeoutMs,
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
        timeoutMs,
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
    active.ready = true;
    Ok(())
}

#[allow(non_snake_case)]
fn initializeService(active: &mut ActiveService, timeoutMs: u64) -> Result<(), String> {
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
    let initializeResponse = readJsonResponse(active, initializeId, timeoutMs)?;
    if initializeResponse.get("error").is_some() {
        return Err(format!("MCP initialize failed: {initializeResponse}"));
    }
    let process = active
        .process
        .as_ref()
        .ok_or_else(|| "Local MCP process is not attached".to_string())?;
    process
        .writeLine(
            &json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            })
            .to_string(),
        )
        .map_err(|error| error.to_string())?;

    let listId = nextRequestId(active);
    let process = active
        .process
        .as_ref()
        .ok_or_else(|| "Local MCP process is not attached".to_string())?;
    process
        .writeLine(
            &json!({
                "jsonrpc": "2.0",
                "id": listId,
                "method": "tools/list",
                "params": {}
            })
            .to_string(),
        )
        .map_err(|error| error.to_string())?;
    let listResponse = readJsonResponse(active, listId, timeoutMs)?;
    if listResponse.get("error").is_some() {
        return Err(format!("MCP tools/list failed: {listResponse}"));
    }
    active.tools = listResponse
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
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
    let body = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    let response = executeRemoteHttp(
        session,
        "POST",
        &session.endpoint,
        buildRemoteHeaders(session, true)?,
        body,
        _timeoutMs,
    )?;
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
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if contentType.contains("text/event-stream")
        || trimmed.lines().any(|line| line.starts_with("data:"))
    {
        return parseSseJsonResponse(trimmed, expectedId);
    }
    let parsed = serde_json::from_str::<Value>(trimmed).map_err(|error| error.to_string())?;
    Ok(Some(parsed))
}

#[allow(non_snake_case)]
fn sendRemoteSseJsonRpc(
    session: &mut RemoteMcpSession,
    payload: Value,
    expectedId: Option<u64>,
    timeoutMs: u64,
) -> Result<Option<Value>, String> {
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
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    loop {
        if Instant::now() >= deadline {
            return Err(format!("Remote MCP SSE request {expectedId} timed out"));
        }
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
    headers.push(("mcp-protocol-version".to_string(), "2024-11-05".to_string()));
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
    let timeoutSeconds = (timeoutMs / 1000).max(1);
    session
        .httpHost
        .executeHttpRequest(HttpRequestData {
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
        })
        .map_err(|error| error.to_string())
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

#[allow(non_snake_case)]
fn readJsonResponse(
    active: &mut ActiveService,
    targetId: u64,
    timeoutMs: u64,
) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    let mut seenIds = BTreeSet::new();
    loop {
        let now = Instant::now();
        if now >= deadline {
            let stderr = active
                .process
                .as_ref()
                .ok_or_else(|| "Local MCP process is not attached".to_string())?
                .drainStderr()
                .unwrap_or_default();
            active.logs.push_str(&stderr);
            return Err(format!("MCP request {targetId} timed out. {stderr}"));
        }
        let waitMs = (deadline - now).as_millis().min(250) as u64;
        let line = active
            .process
            .as_ref()
            .ok_or_else(|| "Local MCP process is not attached".to_string())?
            .readStdoutLine(waitMs)
            .map_err(|error| error.to_string())?;
        let Some(line) = line else {
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
