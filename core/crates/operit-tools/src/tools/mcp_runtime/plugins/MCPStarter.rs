use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use crate::runtime_support::ToolRuntimeSupport;
use crate::tools::mcp_runtime::plugins::MCPBridge::MCPBridge;
use crate::tools::mcp_runtime::plugins::MCPBridgeClient::MCPBridgeClient;
use crate::tools::mcp_runtime::MCPLocalServer::{
    CachedToolInfo, MCPConfig, MCPLocalServer, PluginMetadata,
};
use crate::tools::mcp_runtime::MCPRepository::MCPRepository;
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
    runtimeSupport: Arc<dyn ToolRuntimeSupport>,
}

impl MCPStarter {
    /// Creates an MCP starter bound to one tool runtime.
    pub fn new(context: HostManager, runtimeSupport: Arc<dyn ToolRuntimeSupport>) -> Self {
        Self {
            context,
            runtimeSupport,
        }
    }

    #[allow(non_snake_case)]
    pub fn startPlugin<F>(&self, pluginId: &str, mut statusCallback: F) -> bool
    where
        F: FnMut(StartStatus),
    {
        self.startPluginInternal(
            pluginId,
            MCPBridgeClient::DEFAULT_SPAWN_TIMEOUT_MS,
            &mut statusCallback,
        )
    }

    #[allow(non_snake_case)]
    pub fn startPluginWithTimeout<F>(
        &self,
        pluginId: &str,
        timeoutMs: u64,
        mut statusCallback: F,
    ) -> bool
    where
        F: FnMut(StartStatus),
    {
        self.startPluginInternal(pluginId, timeoutMs, &mut statusCallback)
    }

    #[allow(non_snake_case)]
    pub fn startAllDeployedPlugins(&self) -> (usize, usize, PluginInitStatus) {
        let localServer = MCPLocalServer::getInstance(&self.context);
        let plugins = localServer
            .getAllPluginMetadata()
            .into_keys()
            .filter(|pluginId| localServer.isServerEnabled(pluginId))
            .collect::<Vec<_>>();
        let mut successCount = 0usize;
        for pluginId in &plugins {
            if self.startPlugin(pluginId, |_| {}) {
                successCount += 1;
            }
        }
        (successCount, plugins.len(), PluginInitStatus::SUCCESS)
    }

    #[allow(non_snake_case)]
    pub fn startAllDeployedPluginsWithTimeout(
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
        let mut successCount = 0usize;
        for pluginId in &plugins {
            let startedAtMillis = operit_host_api::TimeUtils::currentTimeMillisU128();
            if self.startPluginWithTimeout(pluginId, timeoutMs, |_| {}) {
                successCount += 1;
            }
            if operit_host_api::TimeUtils::currentTimeMillisU128().saturating_sub(startedAtMillis)
                >= u128::from(timeoutMs)
            {
                break;
            }
        }
        (successCount, plugins.len(), PluginInitStatus::SUCCESS)
    }

    #[allow(non_snake_case)]
    fn startPluginInternal<F>(&self, pluginId: &str, timeoutMs: u64, statusCallback: &mut F) -> bool
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
            let runtimeDir = localServer.getPluginRuntimeDirectory(pluginId);
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
        if !client.connectWithSpawnTimeoutMs(timeoutMs) {
            statusCallback(StartStatus::Error(
                client
                    .getLastConnectionFailureDetail()
                    .unwrap_or_else(|| "Failed to connect to MCP service".to_string()),
            ));
            return false;
        }

        let tools = client.getTools();
        if !tools.is_empty() {
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
            let _ = localServer.cacheServerTools(pluginId.to_string(), cachedTools.clone());
            let _ = bridge.cacheTools(actualServiceName.clone(), tools);
        }

        let pluginInfo = if pluginInfo.description.trim().is_empty() {
            match generateMissingDescription(
                &self.context,
                self.runtimeSupport.clone(),
                pluginId,
                &pluginInfo,
            ) {
                Some(updated) => updated,
                None => pluginInfo,
            }
        } else {
            pluginInfo
        };

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

#[allow(non_snake_case)]
fn generateMissingDescription(
    context: &HostManager,
    runtimeSupport: Arc<dyn ToolRuntimeSupport>,
    pluginId: &str,
    pluginInfo: &PluginMetadata,
) -> Option<PluginMetadata> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    let repository = MCPRepository::getInstance(context, runtimeSupport);
    let generatedDescription = runtime
        .block_on(repository.generatePluginDescription(pluginId, &pluginInfo.name))
        .ok()?;
    if generatedDescription.trim().is_empty() {
        return None;
    }
    let mut updated = pluginInfo.clone();
    updated.description = generatedDescription;
    MCPLocalServer::getInstance(context)
        .addOrUpdatePluginMetadata(pluginId, updated.clone())
        .ok()?;
    Some(updated)
}
