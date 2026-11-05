use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use operit_host_api::{HostManager::HostManager, HttpHost, HttpRequestData, HttpResponseHead};
use operit_plugin_sdk::package::{LocalizedText, PackageTool, PackageToolParameter, ToolPackage};
use operit_store::ExtensionStore::{ExtensionRecord, ExtensionStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const REQUEST_TIMEOUT_MS: u64 = 120_000;
pub const PACKAGE_PREFIX: &str = "a2a.";

/// Portable connection manifest. Credentials follow the installation's explicit scope, like MCP.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2AConfig {
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub card: Value,
}

#[derive(Clone)]
pub struct A2ARepository {
    host: HostManager,
    store: ExtensionStore,
}

impl A2ARepository {
    pub fn new(host: HostManager) -> Result<Self, String> {
        let storage = host
            .runtimeStorageHost
            .clone()
            .ok_or("RuntimeStorageHost is missing")?;
        Ok(Self {
            host,
            store: ExtensionStore::new(storage),
        })
    }

    pub fn records(&self) -> Result<Vec<ExtensionRecord>, String> {
        self.store.records("a2a")
    }

    pub fn config(&self, id: &str) -> Result<A2AConfig, String> {
        let record = self.store.record("a2a", id)?;
        let config: A2AConfig =
            serde_json::from_value(record.settings["agent"].clone()).map_err(|e| e.to_string())?;
        validate_config(&config)?;
        Ok(config)
    }

    /// Validates every definition before changing storage.
    fn definitions(&self, text: &str) -> Result<Vec<(String, A2AConfig)>, String> {
        let document: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let agents = document
            .get("a2aAgents")
            .and_then(Value::as_object)
            .filter(|agents| !agents.is_empty())
            .ok_or("A2A configuration requires a nonempty a2aAgents object")?;
        let records = self.records()?;
        let mut definitions = Vec::new();
        for (id, value) in agents {
            validate_id(id)?;
            let config: A2AConfig =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            validate_config(&config)?;
            if records.iter().filter(|r| r.id == *id).count() > 1 {
                return Err(format!("Extension ownership conflict: a2a:{id}"));
            }
            definitions.push((id.clone(), config));
        }
        Ok(definitions)
    }

    /// Installs the same portable a2aAgents manifest used by files and marketplace entries.
    pub fn import(&self, text: &str) -> Result<Vec<String>, String> {
        let definitions = self.definitions(text)?;
        let mut ids = Vec::new();
        for (id, config) in definitions {
            self.save(&id, config)?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Market updates retain the user's authorization and enabled state in the existing scope.
    pub fn import_market(
        &self,
        text: &str,
        entry_id: &str,
        version_id: &str,
    ) -> Result<Vec<String>, String> {
        if entry_id.trim().is_empty() || version_id.trim().is_empty() {
            return Err("A2A market entry/version is missing".into());
        }
        let mut definitions = self.definitions(text)?;
        let records = self.records()?;
        for (id, config) in &mut definitions {
            if let Some(record) = records.iter().find(|record| record.id == *id) {
                if record.settings["marketEntryId"].as_str() != Some(entry_id) {
                    return Err(format!("A2A {id} already belongs to another installation"));
                }
                let old = self.config(id)?;
                config.disabled = old.disabled;
                // Only carry credentials over when the service origin has not changed.
                if http_url(&old.url)?.origin() == http_url(&config.url)?.origin() {
                    config.headers.extend(old.headers);
                }
            }
        }
        let mut ids = Vec::new();
        for (id, config) in definitions {
            let mut settings = records
                .iter()
                .find(|record| record.id == id)
                .map(|record| record.settings.clone())
                .unwrap_or_else(|| json!({}));
            settings["agent"] = serde_json::to_value(config).map_err(|e| e.to_string())?;
            settings["marketEntryId"] = json!(entry_id);
            settings["installedMarketVersionId"] = json!(version_id);
            if records.iter().any(|record| record.id == id) {
                self.store.setSettings("a2a", &id, settings)?;
            } else {
                self.store.registerDevice("a2a", &id, "", settings)?;
            }
            ids.push(id);
        }
        Ok(ids)
    }

    pub fn save(&self, id: &str, config: A2AConfig) -> Result<(), String> {
        validate_id(id)?;
        validate_config(&config)?;
        if !self.records()?.iter().any(|record| record.id == id) {
            self.store
                .registerDevice("a2a", id, "", json!({"agent": config}))
        } else {
            let mut settings = self.store.record("a2a", id)?.settings;
            settings["agent"] = serde_json::to_value(config).map_err(|e| e.to_string())?;
            self.store.setSettings("a2a", id, settings)
        }
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let mut config = self.config(id)?;
        config.disabled = !enabled;
        self.save(id, config)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.store.delete("a2a", id)
    }

    /// Discovery uses the configured Agent Card URL or the standard well-known location.
    pub async fn discover(&self, mut config: A2AConfig) -> Result<A2AConfig, String> {
        validate_config(&config)?;
        let base = http_url(&config.url)?;
        let card_url = if base.path().ends_with(".json") {
            base.clone()
        } else {
            base.join("/.well-known/agent-card.json")
                .map_err(|e| e.to_string())?
        };
        let card = self
            .request("GET", card_url.as_str(), &config.headers, None)
            .await?;
        let interface = interface(&card)?;
        // A discovered URL must not silently receive credentials intended for a different origin.
        if !config.headers.is_empty() && http_url(&interface.url)?.origin() != card_url.origin() {
            return Err("Agent Card points to a different origin; configure that endpoint explicitly before sending authentication headers".into());
        }
        config.card = card;
        validate_config(&config)?;
        Ok(config)
    }

    /// Performs exactly one request. Send/cancel are never automatically replayed.
    pub async fn invoke(
        &self,
        id: &str,
        operation: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let mut config = self.config(id)?;
        if config.disabled {
            return Err(format!("A2A is disabled: {id}"));
        }
        if config.card.is_null() {
            config = self.discover(config).await?;
            self.save(id, config.clone())?;
        }
        let selected = interface(&config.card)?;
        let input_origin = http_url(&config.url)?.origin();
        if !config.headers.is_empty() && http_url(&selected.url)?.origin() != input_origin {
            return Err(
                "A2A endpoint and authenticated configuration must have the same origin".into(),
            );
        }
        let (method, mut params) = build_operation(&selected.version, operation, &arguments)?;
        if !selected.tenant.is_empty() {
            params["tenant"] = json!(selected.tenant);
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut headers = config.headers.clone();
        headers.retain(|name, _| !name.eq_ignore_ascii_case("a2a-version"));
        headers.insert(
            "A2A-Version".into(),
            selected
                .version
                .split('.')
                .take(2)
                .collect::<Vec<_>>()
                .join("."),
        );
        let response = self
            .request(
                "POST",
                &selected.url,
                &headers,
                Some(json!({"jsonrpc":"2.0", "id":request_id, "method":method, "params":params})),
            )
            .await?;
        if response["jsonrpc"] != "2.0" || response["id"] != request_id {
            return Err("A2A returned a mismatched JSON-RPC response".into());
        }
        if let Some(error) = response.get("error") {
            return Err(format!(
                "A2A error {}: {}",
                error["code"],
                error["message"].as_str().unwrap_or("Unknown error")
            ));
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| "A2A response is missing result".into())
    }

    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: &BTreeMap<String, String>,
        body: Option<Value>,
    ) -> Result<Value, String> {
        http_url(url)?;
        let host = self.host.httpHost.clone().ok_or("HttpHost is missing")?;
        let scheduler = self
            .host
            .hostRuntimeTaskSchedulerHost
            .clone()
            .ok_or("Runtime scheduler is missing")?;
        let started = scheduler.monotonicTimeMillis().map_err(|e| e.to_string())?;
        let mut request_headers = headers
            .iter()
            .filter(|(key, _)| {
                !key.eq_ignore_ascii_case("accept") && !key.eq_ignore_ascii_case("content-type")
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        request_headers.push(("Accept".into(), "application/json".into()));
        if body.is_some() {
            request_headers.push(("Content-Type".into(), "application/json".into()));
        }
        let request = HttpRequestData {
            url: url.into(),
            method: method.into(),
            headers: request_headers,
            body: body
                .map(|v| serde_json::to_vec(&v))
                .transpose()
                .map_err(|e| e.to_string())?
                .unwrap_or_default(),
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 20,
            readTimeoutSeconds: 120,
            followRedirects: false,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        };
        let mut stream = Response::open(host, request)?;
        let mut status = None;
        let mut bytes = Vec::new();
        loop {
            let now = scheduler.monotonicTimeMillis().map_err(|e| e.to_string())?;
            let remaining = REQUEST_TIMEOUT_MS
                .checked_sub(now.saturating_sub(started))
                .filter(|n| *n > 0)
                .ok_or("A2A request timed out")?;
            let event = tokio::select! {
                event = stream.events.recv() => event.ok_or("A2A response stream disconnected")?,
                result = scheduler.waitForHostRuntimeDelay(remaining) => {
                    result.map_err(|e| e.to_string())?;
                    return Err("A2A request timed out".into());
                }
            };
            match event {
                Event::Head(head) if status.is_none() => {
                    if !(200..300).contains(&head.statusCode) {
                        return Err(format!(
                            "A2A HTTP {} {}",
                            head.statusCode, head.statusMessage
                        ));
                    }
                    status = Some(head.statusCode);
                }
                Event::Bytes(chunk) if status.is_some() => {
                    if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                        return Err("A2A response exceeds 8 MiB".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Event::Closed(result) => {
                    result?;
                    if status.is_none() {
                        return Err("A2A response is missing HTTP headers".into());
                    }
                    return serde_json::from_slice(&bytes)
                        .map_err(|e| format!("Invalid A2A JSON response: {e}"));
                }
                _ => return Err("Invalid A2A response stream ordering".into()),
            }
        }
    }
}

struct Interface {
    url: String,
    version: String,
    tenant: String,
}

fn interface(card: &Value) -> Result<Interface, String> {
    if let Some(interfaces) = card["supportedInterfaces"].as_array() {
        for entry in interfaces {
            let version = entry["protocolVersion"].as_str().unwrap_or("");
            if entry["protocolBinding"] == "JSONRPC" && supported_version(version) {
                let url = entry["url"]
                    .as_str()
                    .ok_or("A2A interface URL is missing")?;
                http_url(url)?;
                return Ok(Interface {
                    url: url.into(),
                    version: version.into(),
                    tenant: entry["tenant"].as_str().unwrap_or("").into(),
                });
            }
        }
    } else {
        let version = card["protocolVersion"].as_str().unwrap_or("0.3.0");
        if !supported_version(version) || !version.starts_with("0.3") {
            return Err(format!("Unsupported A2A protocol version: {version}"));
        }
        let mut candidates = Vec::new();
        if card["preferredTransport"].as_str().unwrap_or("JSONRPC") == "JSONRPC" {
            candidates.push(card["url"].as_str().unwrap_or(""));
        }
        if let Some(extra) = card["additionalInterfaces"].as_array() {
            candidates.extend(
                extra
                    .iter()
                    .filter(|entry| entry["transport"] == "JSONRPC")
                    .filter_map(|entry| entry["url"].as_str()),
            );
        }
        for url in candidates {
            if http_url(url).is_ok() {
                return Ok(Interface {
                    url: url.into(),
                    version: version.into(),
                    tenant: String::new(),
                });
            }
        }
    }
    Err("Agent Card has no supported A2A JSONRPC interface (1.0 or 0.3)".into())
}

fn supported_version(version: &str) -> bool {
    let parts = version.split('.').collect::<Vec<_>>();
    matches!(parts.len(), 2 | 3)
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && matches!((parts[0], parts[1]), ("1", "0") | ("0", "3"))
}

pub fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 100
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return Err(
            "A2A id must contain 1–100 ASCII letters, digits, dots, underscores or hyphens".into(),
        );
    }
    Ok(())
}

pub fn validate_config(config: &A2AConfig) -> Result<(), String> {
    http_url(&config.url)?;
    for (key, value) in &config.headers {
        if key.is_empty()
            || !key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
            || value.contains(['\r', '\n'])
        {
            return Err("Invalid A2A HTTP header".into());
        }
    }
    if !config.card.is_null() {
        if config.card["name"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
        {
            return Err("Agent Card name is missing".into());
        }
        interface(&config.card)?;
        if let Some(skills) = config.card.get("skills") {
            let skills = skills
                .as_array()
                .ok_or("Agent Card skills must be an array")?;
            for skill in skills {
                if !skill.is_object()
                    || skill["name"].as_str().is_none()
                    || skill["description"].as_str().is_none()
                {
                    return Err("Agent Card skills require a name and description".into());
                }
            }
        }
    }
    Ok(())
}

fn http_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|e| format!("Invalid A2A URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("A2A requires an HTTP(S) URL without embedded credentials or fragment".into());
    }
    Ok(url)
}

/// The same package identity is used by prompts, search, activation and dispatch.
pub fn package_name(id: &str) -> String {
    format!("{PACKAGE_PREFIX}{id}")
}

/// Identifies runtime-generated connection packages without intercepting ordinary category labels.
pub fn is_a2a_package(package: &ToolPackage) -> bool {
    package.category == "A2A"
        && package.name.starts_with(PACKAGE_PREFIX)
        && package
            .tools
            .first()
            .is_some_and(|tool| tool.script == "/* Operit A2A connection */")
}

fn localized(text: &str) -> LocalizedText {
    LocalizedText {
        values: HashMap::from([("default".into(), text.into())]),
    }
}

pub fn tool_package(id: &str, config: &A2AConfig) -> ToolPackage {
    let parameter = |name: &str, description: &str, required: bool| PackageToolParameter {
        name: name.into(),
        description: localized(description),
        parameter_type: "string".into(),
        required,
    };
    let tool = |name: &str, description: &str, parameters| PackageTool {
        name: name.into(),
        description: localized(description),
        parameters,
        script: "/* Operit A2A connection */".into(),
        ..PackageTool::default()
    };
    let skills = config.card["skills"]
        .as_array()
        .map(|skills| {
            skills
                .iter()
                .map(|s| {
                    format!(
                        "{}: {}",
                        s["name"].as_str().unwrap_or(""),
                        s["description"].as_str().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    ToolPackage {
        name: package_name(id), category: "A2A".into(),
        display_name: localized(config.card["name"].as_str().unwrap_or(id)),
        description: localized(&format!("{}\n{}\nDelegate via A2A. Keep task/context IDs for follow-up; use get_task to check unfinished work.", config.card["description"].as_str().unwrap_or(""), skills)),
        tools: vec![
            tool("send_message", "Send a task or follow-up message to this A2A agent. Returns a message or task with status, IDs and artifacts.", vec![parameter("message", "Task or reply to the agent", true), parameter("task_id", "Existing task ID for continuing a task", false), parameter("context_id", "Existing context ID for related work", false)]),
            tool("get_task", "Retrieve an A2A task's current status, history and artifacts.", vec![parameter("task_id", "Remote task ID", true)]),
            tool("cancel_task", "Request cancellation of an A2A task. Cancellation may be rejected by the agent.", vec![parameter("task_id", "Remote task ID", true)]),
        ], ..ToolPackage::default()
    }
}

pub fn build_operation(
    version: &str,
    operation: &str,
    args: &Value,
) -> Result<(String, Value), String> {
    let legacy = version.starts_with("0.3");
    let required = |name: &str| {
        args[name]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| format!("A2A parameter is required: {name}"))
    };
    match operation {
        "send_message" => {
            let text = required("message")?;
            let mut message = json!({"messageId": uuid::Uuid::new_v4().to_string(), "role": if legacy {"user"} else {"ROLE_USER"}, "parts": if legacy {json!([{"kind":"text", "text":text}])} else {json!([{"text":text}])}});
            if legacy {
                message["kind"] = json!("message");
            }
            for (input, field) in [("task_id", "taskId"), ("context_id", "contextId")] {
                if let Some(value) = args[input].as_str().filter(|v| !v.is_empty()) {
                    message[field] = json!(value);
                }
            }
            Ok((
                if legacy {
                    "message/send"
                } else {
                    "SendMessage"
                }
                .into(),
                json!({"message":message,"configuration": if legacy {json!({"blocking":false})} else {json!({"returnImmediately":true})}}),
            ))
        }
        "get_task" | "cancel_task" => Ok((
            match (legacy, operation) {
                (true, "get_task") => "tasks/get",
                (true, _) => "tasks/cancel",
                (false, "get_task") => "GetTask",
                _ => "CancelTask",
            }
            .into(),
            json!({"id":required("task_id")?}),
        )),
        _ => Err(format!("Unknown A2A operation: {operation}")),
    }
}

enum Event {
    Head(HttpResponseHead),
    Bytes(Vec<u8>),
    Closed(Result<(), String>),
}
struct Response {
    host: Arc<dyn HttpHost>,
    id: String,
    events: tokio::sync::mpsc::UnboundedReceiver<Event>,
}
impl Response {
    fn open(host: Arc<dyn HttpHost>, request: HttpRequestData) -> Result<Self, String> {
        let id = format!("a2a-http-{}", uuid::Uuid::new_v4());
        let (tx, events) = tokio::sync::mpsc::unbounded_channel();
        let head = tx.clone();
        let bytes = tx.clone();
        host.openHttpResponseStream(
            id.clone(),
            request,
            Arc::new(move |h| {
                let _ = head.send(Event::Head(h));
            }),
            Arc::new(move |b| {
                let _ = bytes.send(Event::Bytes(b));
            }),
            Arc::new(move |r| {
                let _ = tx.send(Event::Closed(r));
            }),
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { host, id, events })
    }
}
impl Drop for Response {
    fn drop(&mut self) {
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
