use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Map, Value};
use uuid::Uuid;

use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::data::mcp::plugins::MCPBridge::{MCPBridge, ServiceInfo};

pub struct MCPBridgeClient {
    context: OperitApplicationContext,
    serviceName: String,
    isConnected: AtomicBool,
    lastConnectionFailureDetail: std::sync::Mutex<Option<String>>,
}

impl Clone for MCPBridgeClient {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            serviceName: self.serviceName.clone(),
            isConnected: AtomicBool::new(self.isConnected.load(Ordering::SeqCst)),
            lastConnectionFailureDetail: std::sync::Mutex::new(
                self.getLastConnectionFailureDetail(),
            ),
        }
    }
}

impl MCPBridgeClient {
    pub const DEFAULT_SPAWN_TIMEOUT_MS: u64 = 180_000;

    pub fn new(context: OperitApplicationContext, serviceName: String) -> Self {
        Self {
            context,
            serviceName,
            isConnected: AtomicBool::new(false),
            lastConnectionFailureDetail: std::sync::Mutex::new(None),
        }
    }

    #[allow(non_snake_case)]
    pub fn buildRegisterLocalCommand(
        name: String,
        command: String,
        args: Vec<String>,
        description: Option<String>,
        env: BTreeMap<String, String>,
        cwd: Option<String>,
    ) -> Value {
        let mut params = Map::new();
        params.insert("type".to_string(), Value::String("local".to_string()));
        params.insert("name".to_string(), Value::String(name));
        params.insert("command".to_string(), Value::String(command));
        if !args.is_empty() {
            params.insert(
                "args".to_string(),
                Value::Array(args.into_iter().map(Value::String).collect()),
            );
        }
        if let Some(description) = description {
            params.insert("description".to_string(), Value::String(description));
        }
        if !env.is_empty() {
            params.insert(
                "env".to_string(),
                Value::Object(
                    env.into_iter()
                        .map(|(key, value)| (key, Value::String(value)))
                        .collect(),
                ),
            );
        }
        if let Some(cwd) = cwd {
            params.insert("cwd".to_string(), Value::String(cwd));
        }
        json!({
            "command": "register",
            "id": Uuid::new_v4().to_string(),
            "params": Value::Object(params),
        })
    }

    #[allow(non_snake_case)]
    pub fn buildRegisterRemoteCommand(
        name: String,
        serviceType: String,
        endpoint: String,
        connectionType: Option<String>,
        description: Option<String>,
        bearerToken: Option<String>,
        headers: BTreeMap<String, String>,
    ) -> Value {
        let mut params = Map::new();
        params.insert("type".to_string(), Value::String(serviceType));
        params.insert("name".to_string(), Value::String(name));
        params.insert("endpoint".to_string(), Value::String(endpoint));
        if let Some(connectionType) = connectionType {
            params.insert("connectionType".to_string(), Value::String(connectionType));
        }
        if let Some(description) = description {
            params.insert("description".to_string(), Value::String(description));
        }
        if let Some(bearerToken) = bearerToken {
            params.insert("bearerToken".to_string(), Value::String(bearerToken));
        }
        if !headers.is_empty() {
            params.insert(
                "headers".to_string(),
                Value::Object(
                    headers
                        .into_iter()
                        .map(|(key, value)| (key, Value::String(value)))
                        .collect(),
                ),
            );
        }
        json!({
            "command": "register",
            "id": Uuid::new_v4().to_string(),
            "params": Value::Object(params),
        })
    }

    #[allow(non_snake_case)]
    pub fn connect(&self) -> bool {
        self.connectWithSpawnTimeoutMs(Self::DEFAULT_SPAWN_TIMEOUT_MS)
    }

    #[allow(non_snake_case)]
    pub fn connectWithSpawnTimeoutMs(&self, spawnTimeoutMs: u64) -> bool {
        if self.ping() {
            self.isConnected.store(true, Ordering::SeqCst);
            self.setLastConnectionFailureDetail(None);
            return true;
        }

        let bridge = MCPBridge::getInstance(&self.context);
        let Some(serviceInfo) = bridge.getServiceInfo(&self.serviceName) else {
            self.isConnected.store(false, Ordering::SeqCst);
            self.setLastConnectionFailureDetail(Some(
                "Service is not registered with the bridge.".to_string(),
            ));
            return false;
        };
        if serviceInfo.active && serviceInfo.ready {
            self.isConnected.store(true, Ordering::SeqCst);
            self.setLastConnectionFailureDetail(None);
            return true;
        }

        let spawnResponse =
            bridge.spawnMcpService(&self.context, &self.serviceName, Some(spawnTimeoutMs));
        if spawnResponse
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let ready = spawnResponse
                .get("result")
                .and_then(|result| result.get("ready"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if ready {
                self.isConnected.store(true, Ordering::SeqCst);
                self.setLastConnectionFailureDetail(None);
                return true;
            }
        }
        let message = spawnResponse
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("service not ready")
            .to_string();
        self.isConnected.store(false, Ordering::SeqCst);
        self.setLastConnectionFailureDetail(Some(
            self.buildSpawnFailureDetail(&spawnResponse, &message),
        ));
        false
    }

    #[allow(non_snake_case)]
    pub fn isConnected(&self) -> bool {
        self.isConnected.load(Ordering::SeqCst)
    }

    #[allow(non_snake_case)]
    pub fn ping(&self) -> bool {
        MCPBridge::getInstance(&self.context)
            .getServiceInfo(&self.serviceName)
            .map(|serviceInfo| serviceInfo.active && serviceInfo.ready)
            .unwrap_or(false)
    }

    #[allow(non_snake_case)]
    pub fn spawnBlocking(&self, timeoutMs: u64) -> Value {
        MCPBridge::getInstance(&self.context).spawnMcpService(
            &self.context,
            &self.serviceName,
            Some(timeoutMs),
        )
    }

    #[allow(non_snake_case)]
    pub fn unspawn(&self) -> bool {
        let result = MCPBridge::getInstance(&self.context).unspawnMcpService(&self.serviceName);
        if result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            self.disconnect();
            return true;
        }
        false
    }

    #[allow(non_snake_case)]
    pub fn isActive(&self) -> bool {
        self.getServiceInfo()
            .map(|info| info.active)
            .unwrap_or(false)
    }

    #[allow(non_snake_case)]
    pub fn callTool(&self, method: &str, params: Value) -> Value {
        if !self.isConnected() && !self.connect() {
            return json!({
                "success": false,
                "error": {
                    "code": -1,
                    "message": format!("Cannot connect to MCP service {}", self.serviceName)
                }
            });
        }
        let retryParams = params.clone();
        let response =
            MCPBridge::getInstance(&self.context).callTool(&self.serviceName, method, params);
        if response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return response;
        }
        let errorMessage = response
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if errorMessage.contains("not active") || errorMessage.contains("timeout") {
            self.isConnected.store(false, Ordering::SeqCst);
            if self.connect() {
                return MCPBridge::getInstance(&self.context).callTool(
                    &self.serviceName,
                    method,
                    retryParams,
                );
            }
        }
        response
    }

    #[allow(non_snake_case)]
    pub fn callToolSync(&self, method: &str, params: BTreeMap<String, Value>) -> Value {
        self.callTool(
            method,
            Value::Object(params.into_iter().collect::<Map<String, Value>>()),
        )
    }

    #[allow(non_snake_case)]
    pub fn getTools(&self) -> Vec<Value> {
        if !self.isConnected() && !self.connect() {
            return Vec::new();
        }
        let response = MCPBridge::getInstance(&self.context).listTools(&self.serviceName);
        if !response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            self.isConnected.store(false, Ordering::SeqCst);
            return Vec::new();
        }
        response
            .get("result")
            .and_then(|result| result.get("tools"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn getServiceInfo(&self) -> Option<ServiceInfo> {
        MCPBridge::getInstance(&self.context).getServiceInfo(&self.serviceName)
    }

    #[allow(non_snake_case)]
    pub fn getToolDescriptions(&self) -> Vec<String> {
        self.getTools()
            .into_iter()
            .filter_map(|tool| {
                let name = tool.get("name").and_then(Value::as_str).unwrap_or_default();
                if name.is_empty() {
                    return None;
                }
                let description = tool
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if description.is_empty() {
                    Some(name.to_string())
                } else {
                    Some(format!("{name}: {description}"))
                }
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn disconnect(&self) {
        self.isConnected.store(false, Ordering::SeqCst);
    }

    #[allow(non_snake_case)]
    pub fn getLastConnectionFailureDetail(&self) -> Option<String> {
        self.lastConnectionFailureDetail
            .lock()
            .expect("mcp client failure mutex poisoned")
            .clone()
    }

    #[allow(non_snake_case)]
    fn setLastConnectionFailureDetail(&self, detail: Option<String>) {
        *self
            .lastConnectionFailureDetail
            .lock()
            .expect("mcp client failure mutex poisoned") = detail.and_then(|text| {
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });
    }

    #[allow(non_snake_case)]
    fn buildSpawnFailureDetail(&self, spawnResponse: &Value, fallbackMessage: &str) -> String {
        let error = spawnResponse.get("error");
        let errorMessage = error
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .unwrap_or(fallbackMessage);
        let data = error.and_then(|value| value.get("data"));
        let lastError = data
            .and_then(|value| value.get("lastError"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let logs = data
            .and_then(|value| value.get("logs"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let logSnippet = logs
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" | ");
        let mut detail = errorMessage.to_string();
        if !lastError.is_empty() && !detail.contains(lastError) {
            detail.push_str(" Last error: ");
            detail.push_str(lastError);
        }
        if !logSnippet.is_empty() {
            detail.push_str(" Logs: ");
            detail.push_str(&logSnippet.chars().take(500).collect::<String>());
        }
        detail
    }
}
