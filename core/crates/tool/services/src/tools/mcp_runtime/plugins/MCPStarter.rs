use std::collections::BTreeMap;

use serde_json::Value;

use crate::tools::mcp_runtime::plugins::MCPBridge::MCPBridge;
use crate::tools::mcp_runtime::plugins::MCPBridgeClient::MCPBridgeClient;
use crate::tools::mcp_runtime::MCPLocalServer::{CachedToolInfo, MCPConfig, MCPLocalServer};
use crate::tools::PackageLoadingProgress::{
    appendPluginLoadingItemLog, ensurePluginLoadingItem, markPluginLoadingItemFailed,
    markPluginLoadingItemLoading, markPluginLoadingItemSuccess, pluginLoadingSessionActive,
    PLUGIN_LOAD_KIND_MCP,
};
use operit_host_api::HostManager::HostManager;
use operit_tools::tools::mcp::MCPManager::MCPManager;
use operit_tools::tools::mcp::MCPServerConfig::MCPServerConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginInitStatus {
    SUCCESS,
    TERMINAL_SERVICE_UNAVAILABLE,
    NODEJS_MISSING,
    BRIDGE_FAILED,
    OTHER_ERROR,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartStatus {
    InProgress(String),
    Success(String),
    Error(String),
    TerminalServiceUnavailable(String),
    PnpmMissing(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationResult {
    pub pluginId: String,
    pub serviceName: String,
    pub isResponding: bool,
    pub responseTime: i64,
    pub details: String,
}

#[derive(Clone)]
pub struct MCPStarter {
    context: HostManager,
}

impl MCPStarter {
    /// Creates an MCP starter using host capabilities without model dependencies.
    pub fn new(context: HostManager) -> Self {
        Self { context }
    }

    /// Starts one enabled MCP server without issuing model requests.
    #[allow(non_snake_case)]
    pub async fn startPlugin<F>(&self, pluginId: &str, mut statusCallback: F) -> bool
    where
        F: FnMut(StartStatus),
    {
        self.startPluginInternal(
            pluginId,
            MCPBridgeClient::DEFAULT_SPAWN_TIMEOUT_MS,
            &mut statusCallback,
        ).await
    }

    /// Starts one enabled MCP server with a connection timeout.
    #[allow(non_snake_case)]
    pub async fn startPluginWithTimeout<F>(
        &self,
        pluginId: &str,
        timeoutMs: u64,
        mut statusCallback: F,
    ) -> bool
    where
        F: FnMut(StartStatus),
    {
        self.startPluginInternal(pluginId, timeoutMs, &mut statusCallback).await
    }

    /// Attempts every enabled server and reports aggregate failures.
    #[allow(non_snake_case)]
    pub async fn startAllDeployedPlugins(&self) -> (usize, usize, PluginInitStatus) {
        let localServer = MCPLocalServer::getInstance(&self.context);
        let plugins = localServer
            .getAllPluginMetadata()
            .into_keys()
            .filter(|pluginId| localServer.isServerEnabled(pluginId))
            .collect::<Vec<_>>();
        startEveryPlugin(&plugins, |pluginId| async move { self.startPlugin(&pluginId, |_| {}).await }).await
    }

    /// Applies an independent timeout to each enabled server.
    #[allow(non_snake_case)]
    pub async fn startAllDeployedPluginsWithTimeout(
        &self,
        timeoutSeconds: i32,
    ) -> (usize, usize, PluginInitStatus) {
        let localServer = MCPLocalServer::getInstance(&self.context);
        let plugins = localServer
            .getAllPluginMetadata()
            .into_keys()
            .filter(|pluginId| localServer.isServerEnabled(pluginId))
            .collect::<Vec<_>>();
        let timeoutMs = timeoutSeconds.max(1) as u64 * 1000;
        for pluginId in &plugins {
            reportMcpPluginQueued(pluginId, &localServer);
        }
        startEveryPlugin(&plugins, |pluginId| {
            let localServer = &localServer;
            async move {
            reportMcpPluginStarting(&pluginId, &localServer);
            let mut lastError = String::new();
            let started = self.startPluginWithTimeout(&pluginId, timeoutMs, |status| {
                let message = startStatusMessage(&status);
                appendMcpPluginLog(&pluginId, &message);
                if matches!(
                    &status,
                    StartStatus::Error(_)
                        | StartStatus::TerminalServiceUnavailable(_)
                        | StartStatus::PnpmMissing(_)
                ) {
                    lastError = message;
                }
            }).await;
            if started {
                reportMcpPluginSuccess(&pluginId);
            } else {
                reportMcpPluginFailure(&pluginId, &lastError);
            }
            started
            }
        }).await
    }

    /// Connects a server and publishes its discovered tools.
    #[allow(non_snake_case)]
    async fn startPluginInternal<F>(&self, pluginId: &str, timeoutMs: u64, statusCallback: &mut F) -> bool
    where
        F: FnMut(StartStatus),
    {
        let localServer = MCPLocalServer::getInstance(&self.context);
        let Some(pluginInfo) = localServer.getPluginMetadata(pluginId) else {
            statusCallback(StartStatus::Error(format!(
                "Plugin info not found: {pluginId}"
            )));
            return false;
        };
        if !localServer.isServerEnabled(pluginId) {
            statusCallback(StartStatus::Error(format!(
                "Plugin not enabled by user: {pluginId}"
            )));
            return false;
        }

        statusCallback(StartStatus::InProgress(format!(
            "Starting plugin: {pluginId}"
        )));
        let bridge = MCPBridge::getInstance(&self.context);
        let serverName = pluginId.to_string();
        let mut actualServiceName = serverName.clone();
        let serverConfig = localServer.getMCPServer(pluginId);
        let registerResult = if serverConfig
            .as_ref()
            .and_then(|config| config.url.as_ref())
            .map(|url| !url.trim().is_empty())
            .unwrap_or(false)
        {
            let serverConfig = serverConfig.clone().unwrap();
            let endpoint = serverConfig.url.clone().unwrap();
            bridge.registerRemoteMcpService(
                serverName.clone(),
                endpoint,
                serverConfig.r#type.clone(),
                Some(format!("Remote MCP Server: {pluginId}")),
                None,
                serverConfig.headers.clone(),
            )
        } else {
            let pluginConfig = localServer.getPluginConfig(pluginId);
            let extractedServerName = extractServerNameFromConfig(&pluginConfig);
            let config = parseConfigJson(&pluginConfig);
            actualServiceName = match extractedServerName {
                Some(value) => value,
                None => serverName.clone(),
            };
            let serverConfig = config
                .and_then(|config| config.mcpServers.get(&actualServiceName).cloned())
                .or_else(|| localServer.getMCPServer(pluginId));
            let Some(serverConfig) = serverConfig else {
                statusCallback(StartStatus::Error(format!(
                    "Invalid plugin config: {pluginId}"
                )));
                return false;
            };
            let runtimeDir = match localServer.preparePluginRuntimeDirectory(pluginId) {
                Ok(directory) => directory,
                Err(error) => {
                    statusCallback(StartStatus::Error(error));
                    return false;
                }
            };
            bridge.registerMcpService(
                actualServiceName.clone(),
                serverConfig.command.clone(),
                serverConfig.args.clone(),
                Some(format!("MCP Server: {pluginId}")),
                serverConfig.env.clone(),
                Some(runtimeDir),
            )
        };
        if !registerResult
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let message = registerResult
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Failed to register MCP service")
                .to_string();
            statusCallback(StartStatus::Error(message));
            return false;
        }

        let client = MCPBridgeClient::new(self.context.clone(), actualServiceName.clone());
        if !client.connectWithSpawnTimeoutMs(timeoutMs).await {
            statusCallback(StartStatus::Error(
                client
                    .getLastConnectionFailureDetail()
                    .unwrap_or_else(|| "Failed to connect to MCP service".to_string()),
            ));
            return false;
        }

        let tools = client.getTools();
        let cachedTools = tools
            .iter()
            .map(|tool| CachedToolInfo {
                name: tool
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                inputSchema: tool
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}))
                    .to_string(),
                cachedAt: currentTimeMillis(),
            })
            .collect::<Vec<_>>();
        if let Err(error) = localServer.cacheServerTools(pluginId.to_string(), cachedTools) {
            statusCallback(StartStatus::Error(format!(
                "Failed to persist MCP tools: {error}"
            )));
            return false;
        }
        let _ = bridge.cacheTools(actualServiceName.clone(), tools);

        MCPManager::getInstance(self.context.clone()).registerServer(
            actualServiceName.clone(),
            MCPServerConfig {
                name: actualServiceName.clone(),
                endpoint: if serverConfig
                    .as_ref()
                    .and_then(|config| config.url.as_ref())
                    .map(|url| !url.trim().is_empty())
                    .unwrap_or(false)
                {
                    serverConfig
                        .and_then(|config| config.url)
                        .unwrap_or_default()
                } else {
                    format!("mcp://plugin/{actualServiceName}")
                },
                description: pluginInfo.description,
                capabilities: vec!["tools".to_string()],
                extraData: BTreeMap::new(),
            },
        );
        statusCallback(StartStatus::Success(format!(
            "Service {pluginId} started successfully"
        )));
        true
    }
}

#[allow(non_snake_case)]
fn extractServerNameFromConfig(configJson: &str) -> Option<String> {
    if configJson.trim().is_empty() {
        return None;
    }
    let value = serde_json::from_str::<Value>(configJson).ok()?;
    value
        .get("mcpServers")
        .and_then(Value::as_object)?
        .keys()
        .next()
        .cloned()
}

#[allow(non_snake_case)]
fn parseConfigJson(configJson: &str) -> Option<MCPConfig> {
    if configJson.trim().is_empty() {
        return None;
    }
    serde_json::from_str::<MCPConfig>(configJson).ok()
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}

fn mcpPluginDisplayName(pluginId: &str, localServer: &MCPLocalServer) -> String {
    localServer
        .getPluginMetadata(pluginId)
        .map(|metadata| metadata.name)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| pluginId.split('/').last().unwrap_or(pluginId).to_string())
}

fn startStatusMessage(status: &StartStatus) -> String {
    match status {
        StartStatus::InProgress(message)
        | StartStatus::Success(message)
        | StartStatus::Error(message)
        | StartStatus::TerminalServiceUnavailable(message)
        | StartStatus::PnpmMissing(message) => message.clone(),
    }
}

fn reportMcpPluginQueued(pluginId: &str, localServer: &MCPLocalServer) {
    if !pluginLoadingSessionActive() {
        return;
    }
    ensurePluginLoadingItem(
        pluginId,
        &mcpPluginDisplayName(pluginId, localServer),
        PLUGIN_LOAD_KIND_MCP,
    );
}

fn reportMcpPluginStarting(pluginId: &str, localServer: &MCPLocalServer) {
    if !pluginLoadingSessionActive() {
        return;
    }
    let displayName = mcpPluginDisplayName(pluginId, localServer);
    ensurePluginLoadingItem(pluginId, &displayName, PLUGIN_LOAD_KIND_MCP);
    markPluginLoadingItemLoading(pluginId, Some(&displayName));
}

fn appendMcpPluginLog(pluginId: &str, message: &str) {
    if !pluginLoadingSessionActive() {
        return;
    }
    appendPluginLoadingItemLog(pluginId, message);
}

fn reportMcpPluginSuccess(pluginId: &str) {
    if !pluginLoadingSessionActive() {
        return;
    }
    markPluginLoadingItemSuccess(pluginId, None);
}

fn reportMcpPluginFailure(pluginId: &str, message: &str) {
    if !pluginLoadingSessionActive() {
        return;
    }
    let failure = if message.trim().is_empty() {
        "failed"
    } else {
        message
    };
    markPluginLoadingItemFailed(pluginId, failure, message);
}

/// Attempts every queued server exactly once and summarizes the observed results.
#[allow(non_snake_case)]
async fn startEveryPlugin<F, Fut>(plugins: &[String], mut start: F) -> (usize, usize, PluginInitStatus)
where F: FnMut(String) -> Fut, Fut: std::future::Future<Output = bool> {
    let mut successCount = 0;
    for plugin in plugins {
        if start(plugin.clone()).await { successCount += 1; }
    }
    let status = if successCount == plugins.len() { PluginInitStatus::SUCCESS } else { PluginInitStatus::OTHER_ERROR };
    (successCount, plugins.len(), status)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keeps later servers eligible after the first server fails or times out.
    #[tokio::test]
    async fn failedServerDoesNotTruncateStartup() {
        let plugins = vec!["failed".to_string(), "ready".to_string()];
        let mut attempted = Vec::new();
        let result = startEveryPlugin(&plugins, |plugin| {
            attempted.push(plugin.to_string());
            std::future::ready(plugin == "ready")
        }).await;
        assert_eq!(attempted, plugins);
        assert_eq!(result, (1, 2, PluginInitStatus::OTHER_ERROR));
    }

    /// Reports success only when every queued server starts successfully.
    #[tokio::test]
    async fn successfulBatchReportsExactCounts() {
        let plugins = vec!["one".to_string(), "two".to_string()];
        assert_eq!(
            startEveryPlugin(&plugins, |_| std::future::ready(true)).await,
            (2, 2, PluginInitStatus::SUCCESS)
        );
        assert_eq!(
            startEveryPlugin(&plugins, |_| std::future::ready(false)).await,
            (0, 2, PluginInitStatus::OTHER_ERROR)
        );
        assert_eq!(
            startEveryPlugin(&[], |_| std::future::ready(panic!("empty batch"))).await,
            (0, 0, PluginInitStatus::SUCCESS)
        );
    }
}
