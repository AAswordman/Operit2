use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Clone, Debug)]
pub struct MCPLocalServer {
    storePaths: RuntimeStorePaths,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPConfig {
    #[serde(rename = "mcpServers", default)]
    pub mcpServers: BTreeMap<String, ServerConfig>,
    #[serde(rename = "pluginMetadata", default)]
    pub pluginMetadata: BTreeMap<String, PluginMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(rename = "autoApprove", default)]
    pub autoApprove: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "logoUrl", default)]
    pub logoUrl: Option<String>,
    #[serde(default = "unknownAuthor")]
    pub author: String,
    #[serde(rename = "isInstalled", default)]
    pub isInstalled: bool,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "updatedAt", default)]
    pub updatedAt: String,
    #[serde(rename = "longDescription", default)]
    pub longDescription: String,
    #[serde(rename = "repoUrl", default)]
    pub repoUrl: String,
    #[serde(default = "localType")]
    pub r#type: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(rename = "connectionType", default = "httpStreamConnectionType")]
    pub connectionType: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(rename = "bearerToken", default)]
    pub bearerToken: Option<String>,
    #[serde(default)]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(rename = "installedPath", default)]
    pub installedPath: Option<String>,
    #[serde(rename = "installedTime", default = "currentTimeMillis")]
    pub installedTime: i64,
    #[serde(rename = "marketConfig", default)]
    pub marketConfig: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerStatus {
    #[serde(rename = "serverId")]
    pub serverId: String,
    #[serde(rename = "lastStartTime", default)]
    pub lastStartTime: i64,
    #[serde(rename = "lastStopTime", default)]
    pub lastStopTime: i64,
    #[serde(rename = "errorMessage", default)]
    pub errorMessage: Option<String>,
    #[serde(rename = "cachedTools", default)]
    pub cachedTools: Option<Vec<CachedToolInfo>>,
    #[serde(rename = "toolsCachedTime", default)]
    pub toolsCachedTime: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "inputSchema", default = "emptyJsonObjectString")]
    pub inputSchema: String,
    #[serde(rename = "cachedAt", default = "currentTimeMillis")]
    pub cachedAt: i64,
}

struct SanitizedConfigResult {
    config: MCPConfig,
    removedServerIds: Vec<String>,
    removedMetadataIds: Vec<String>,
}

impl MCPLocalServer {
    #[allow(non_snake_case)]
    pub fn getInstance(_context: &OperitApplicationContext) -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    pub fn new(storePaths: RuntimeStorePaths) -> Self {
        let server = Self { storePaths };
        let _ = server.storePaths.ensure_mcp_plugins_dir();
        let _ = server.loadAllConfigurations();
        server
    }

    #[allow(non_snake_case)]
    pub fn reloadConfigurations(&self) -> Result<(), String> {
        self.loadAllConfigurations()
    }

    #[allow(non_snake_case)]
    fn loadAllConfigurations(&self) -> Result<(), String> {
        self.storePaths
            .ensure_mcp_plugins_dir()
            .map_err(|error| error.to_string())?;
        let config = self.readMCPConfig()?;
        let sanitized = self.sanitizeMCPConfig(config, "loadAllConfigurations");
        let updatedConfig = self.autoFillMissingMetadata(sanitized.config.clone());
        if updatedConfig != sanitized.config
            || !sanitized.removedServerIds.is_empty()
            || !sanitized.removedMetadataIds.is_empty()
        {
            self.writeMCPConfig(&updatedConfig)?;
        }

        let mut status = self.readServerStatus()?;
        let mut changed = false;
        for serverId in updatedConfig.mcpServers.keys() {
            if !status.contains_key(serverId) {
                status.insert(
                    serverId.clone(),
                    ServerStatus {
                        serverId: serverId.clone(),
                        lastStartTime: 0,
                        lastStopTime: 0,
                        errorMessage: None,
                        cachedTools: None,
                        toolsCachedTime: 0,
                    },
                );
                changed = true;
            }
        }
        if changed {
            self.writeServerStatus(&status)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn saveMCPConfig(&self) -> Result<(), String> {
        let config = self.readMCPConfig()?;
        self.writeMCPConfig(&config)
    }

    #[allow(non_snake_case)]
    pub fn saveServerStatus(&self) -> Result<(), String> {
        let status = self.readServerStatus()?;
        self.writeServerStatus(&status)
    }

    #[allow(non_snake_case)]
    pub fn addOrUpdateMCPServer(
        &self,
        serverId: String,
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        disabled: bool,
        autoApprove: Vec<String>,
    ) -> Result<(), String> {
        let normalizedCommand = command.trim().to_string();
        if normalizedCommand.is_empty() {
            return Err(format!("MCP server {serverId} command cannot be empty"));
        }

        let mut config = self.readMCPConfig()?;
        config.mcpServers.insert(
            serverId,
            ServerConfig {
                command: normalizedCommand,
                args: args
                    .into_iter()
                    .filter(|item| !item.trim().is_empty())
                    .collect(),
                disabled,
                autoApprove: autoApprove
                    .into_iter()
                    .filter(|item| !item.trim().is_empty())
                    .collect(),
                env: cleanEnv(env),
            },
        );
        self.writeMCPConfig(&config)
    }

    #[allow(non_snake_case)]
    pub fn removeMCPServer(&self, serverId: &str) -> Result<(), String> {
        let mut config = self.readMCPConfig()?;
        config.mcpServers.remove(serverId);
        config.pluginMetadata.remove(serverId);
        self.writeMCPConfig(&config)?;
        self.removeServerStatus(serverId)
    }

    #[allow(non_snake_case)]
    pub fn mergeConfigFromJson(&self, jsonConfig: &str) -> Result<usize, String> {
        let parsedConfig = serde_json::from_str::<MCPConfig>(jsonConfig)
            .map_err(|error| format!("JSON format error: {error}"))?;
        if parsedConfig.mcpServers.is_empty() {
            return Err("No mcpServers field or mcpServers is empty".to_string());
        }
        let sanitized = self.sanitizeMCPConfig(parsedConfig, "mergeConfigFromJson");
        if sanitized.config.mcpServers.is_empty() {
            return Err("mcpServers is empty".to_string());
        }

        let mut current = self.readMCPConfig()?;
        let mut addedCount = 0usize;
        for (serverId, serverConfig) in sanitized.config.mcpServers {
            current.mcpServers.insert(serverId, serverConfig);
            addedCount += 1;
        }
        current = self.autoFillMissingMetadata(current);
        self.writeMCPConfig(&current)?;
        self.initializeMissingServerStatus()?;
        Ok(addedCount)
    }

    #[allow(non_snake_case)]
    pub fn getConfigFilePath(&self) -> String {
        self.storePaths
            .mcp_config_path()
            .to_string_lossy()
            .to_string()
    }

    #[allow(non_snake_case)]
    pub fn getConfigDirectory(&self) -> String {
        self.storePaths
            .mcp_plugins_dir()
            .to_string_lossy()
            .to_string()
    }

    #[allow(non_snake_case)]
    pub fn getMCPServer(&self, serverId: &str) -> Option<ServerConfig> {
        self.readMCPConfig().ok()?.mcpServers.get(serverId).cloned()
    }

    #[allow(non_snake_case)]
    pub fn getAllMCPServers(&self) -> BTreeMap<String, ServerConfig> {
        self.readMCPConfig()
            .map(|config| config.mcpServers)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn addOrUpdatePluginMetadata(&self, metadata: PluginMetadata) -> Result<(), String> {
        let mut config = self.readMCPConfig()?;
        config.pluginMetadata.insert(metadata.id.clone(), metadata);
        self.writeMCPConfig(&config)
    }

    #[allow(non_snake_case)]
    pub fn removePluginMetadata(&self, pluginId: &str) -> Result<(), String> {
        let mut config = self.readMCPConfig()?;
        config.pluginMetadata.remove(pluginId);
        self.writeMCPConfig(&config)
    }

    #[allow(non_snake_case)]
    pub fn getPluginMetadata(&self, pluginId: &str) -> Option<PluginMetadata> {
        self.readMCPConfig()
            .ok()?
            .pluginMetadata
            .get(pluginId)
            .cloned()
    }

    #[allow(non_snake_case)]
    pub fn getAllPluginMetadata(&self) -> BTreeMap<String, PluginMetadata> {
        self.readMCPConfig()
            .map(|config| config.pluginMetadata)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn updateServerStatus(
        &self,
        serverId: String,
        errorMessage: Option<String>,
        cachedTools: Option<Vec<CachedToolInfo>>,
        lastStartTime: Option<i64>,
        lastStopTime: Option<i64>,
    ) -> Result<(), String> {
        let mut statusMap = self.readServerStatus()?;
        let existing = statusMap.get(&serverId).cloned().unwrap_or(ServerStatus {
            serverId: serverId.clone(),
            lastStartTime: 0,
            lastStopTime: 0,
            errorMessage: None,
            cachedTools: None,
            toolsCachedTime: 0,
        });
        let hasCachedTools = cachedTools.is_some();
        statusMap.insert(
            serverId.clone(),
            ServerStatus {
                serverId,
                errorMessage: errorMessage.or(existing.errorMessage),
                cachedTools: cachedTools.or(existing.cachedTools),
                toolsCachedTime: if hasCachedTools {
                    currentTimeMillis()
                } else {
                    existing.toolsCachedTime
                },
                lastStartTime: lastStartTime.unwrap_or(existing.lastStartTime),
                lastStopTime: lastStopTime.unwrap_or(existing.lastStopTime),
            },
        );
        self.writeServerStatus(&statusMap)
    }

    #[allow(non_snake_case)]
    pub fn cacheServerTools(
        &self,
        serverId: String,
        tools: Vec<CachedToolInfo>,
    ) -> Result<(), String> {
        self.updateServerStatus(serverId, None, Some(tools), None, None)
    }

    #[allow(non_snake_case)]
    pub fn getCachedTools(&self, serverId: &str) -> Option<Vec<CachedToolInfo>> {
        self.readServerStatus()
            .ok()?
            .get(serverId)
            .and_then(|status| status.cachedTools.clone())
    }

    #[allow(non_snake_case)]
    pub fn hasValidToolCache(&self, serverId: &str) -> bool {
        let Some(status) = self
            .readServerStatus()
            .ok()
            .and_then(|map| map.get(serverId).cloned())
        else {
            return false;
        };
        let Some(tools) = status.cachedTools else {
            return false;
        };
        if tools.is_empty() || status.toolsCachedTime <= 0 {
            return false;
        }
        currentTimeMillis() - status.toolsCachedTime < 24 * 60 * 60 * 1000
    }

    #[allow(non_snake_case)]
    pub fn removeServerStatus(&self, serverId: &str) -> Result<(), String> {
        let mut statusMap = self.readServerStatus()?;
        statusMap.remove(serverId);
        self.writeServerStatus(&statusMap)
    }

    #[allow(non_snake_case)]
    pub fn getServerStatus(&self, serverId: &str) -> Option<ServerStatus> {
        self.readServerStatus().ok()?.get(serverId).cloned()
    }

    #[allow(non_snake_case)]
    pub fn getAllServerStatus(&self) -> BTreeMap<String, ServerStatus> {
        self.readServerStatus().unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn isServerLikelyRunning(&self, serverId: &str) -> bool {
        let Some(status) = self.getServerStatus(serverId) else {
            return false;
        };
        status.lastStartTime > 0 && status.lastStartTime >= status.lastStopTime
    }

    #[allow(non_snake_case)]
    pub fn isServerEnabled(&self, serverId: &str) -> bool {
        if let Some(serverConfig) = self.getMCPServer(serverId) {
            return !serverConfig.disabled;
        }
        if let Some(metadata) = self.getPluginMetadata(serverId) {
            if metadata.r#type == "remote" {
                return !metadata.disabled;
            }
        }
        true
    }

    #[allow(non_snake_case)]
    pub fn setServerEnabled(&self, serverId: &str, enabled: bool) -> Result<(), String> {
        let mut config = self.readMCPConfig()?;
        if let Some(serverConfig) = config.mcpServers.get_mut(serverId) {
            serverConfig.disabled = !enabled;
            return self.writeMCPConfig(&config);
        }
        if let Some(metadata) = config.pluginMetadata.get_mut(serverId) {
            if metadata.r#type == "remote" {
                metadata.disabled = !enabled;
                return self.writeMCPConfig(&config);
            }
        }
        Err(format!(
            "Cannot set enabled state, server config or remote metadata not found: {serverId}"
        ))
    }

    #[allow(non_snake_case)]
    pub fn getPluginRuntimeDirectory(&self, pluginId: &str) -> String {
        self.storePaths
            .mcp_plugins_dir()
            .join(pluginId.split('/').last().unwrap_or(pluginId))
            .to_string_lossy()
            .to_string()
    }

    #[allow(non_snake_case)]
    pub fn isPluginRuntimeReady(&self, pluginId: &str) -> bool {
        let Some(metadata) = self.getPluginMetadata(pluginId) else {
            return false;
        };
        if metadata.r#type == "remote" {
            return true;
        }
        let dir = PathBuf::from(self.getPluginRuntimeDirectory(pluginId));
        if !dir.is_dir() {
            return false;
        }
        if !self.pluginRuntimeRequiresFiles(pluginId) {
            return true;
        }
        fs::read_dir(dir)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    }

    #[allow(non_snake_case)]
    pub fn getPluginConfig(&self, pluginId: &str) -> String {
        if let Some(serverConfig) = self.getMCPServer(pluginId) {
            let mut config = MCPConfig::default();
            config.mcpServers.insert(pluginId.to_string(), serverConfig);
            return serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string());
        }
        serde_json::to_string_pretty(&MCPConfig::default()).unwrap_or_else(|_| "{}".to_string())
    }

    #[allow(non_snake_case)]
    pub fn savePluginConfig(&self, pluginId: &str, configJson: &str) -> Result<bool, String> {
        let parsedServerConfig = serde_json::from_str::<MCPConfig>(configJson)
            .ok()
            .and_then(|config| config.mcpServers.get(pluginId).cloned())
            .or_else(|| serde_json::from_str::<ServerConfig>(configJson).ok());
        let Some(serverConfig) = parsedServerConfig else {
            return Ok(false);
        };
        let Some(sanitizedServer) =
            self.sanitizeServerConfig(pluginId, serverConfig, "savePluginConfig")
        else {
            return Ok(false);
        };
        let mut config = self.readMCPConfig()?;
        config
            .mcpServers
            .insert(pluginId.to_string(), sanitizedServer);
        self.writeMCPConfig(&config)?;
        Ok(true)
    }

    #[allow(non_snake_case)]
    pub fn exportConfigAsJson(&self) -> String {
        serde_json::json!({
            "mcpConfig": self.readMCPConfig().unwrap_or_default(),
            "serverStatus": self.readServerStatus().unwrap_or_default(),
            "exportTime": currentTimeMillis(),
            "version": "1.0"
        })
        .to_string()
    }

    #[allow(non_snake_case)]
    pub fn importConfigFromJson(&self, json: &str) -> Result<bool, String> {
        let value =
            serde_json::from_str::<serde_json::Value>(json).map_err(|error| error.to_string())?;
        if let Some(configValue) = value.get("mcpConfig") {
            let rawConfig = serde_json::from_value::<MCPConfig>(configValue.clone())
                .map_err(|error| error.to_string())?;
            let sanitized = self.sanitizeMCPConfig(rawConfig, "importConfigFromJson");
            self.writeMCPConfig(&self.autoFillMissingMetadata(sanitized.config))?;
        }
        if let Some(statusValue) = value.get("serverStatus") {
            let status =
                serde_json::from_value::<BTreeMap<String, ServerStatus>>(statusValue.clone())
                    .map_err(|error| error.to_string())?;
            self.writeServerStatus(&status)?;
        }
        Ok(true)
    }

    #[allow(non_snake_case)]
    pub fn cleanupInvalidConfigurations(&self) -> Result<(), String> {
        let mut config = self.readMCPConfig()?;
        let validPluginIds = config.pluginMetadata.keys().cloned().collect::<Vec<_>>();
        config
            .mcpServers
            .retain(|serverId, _| validPluginIds.iter().any(|id| id == serverId));
        self.writeMCPConfig(&config)?;
        let mut status = self.readServerStatus()?;
        status.retain(|serverId, _| validPluginIds.iter().any(|id| id == serverId));
        self.writeServerStatus(&status)
    }

    #[allow(non_snake_case)]
    fn pluginRuntimeRequiresFiles(&self, pluginId: &str) -> bool {
        match self.getPluginCommandName(pluginId).as_deref() {
            Some("npx") | Some("uvx") | Some("uv") => false,
            _ => true,
        }
    }

    #[allow(non_snake_case)]
    fn getPluginCommandName(&self, pluginId: &str) -> Option<String> {
        self.getMCPServer(pluginId).map(|config| {
            config
                .command
                .trim()
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase()
        })
    }

    #[allow(non_snake_case)]
    fn initializeMissingServerStatus(&self) -> Result<(), String> {
        let config = self.readMCPConfig()?;
        let mut status = self.readServerStatus()?;
        let mut changed = false;
        for serverId in config.mcpServers.keys() {
            if !status.contains_key(serverId) {
                status.insert(
                    serverId.clone(),
                    ServerStatus {
                        serverId: serverId.clone(),
                        lastStartTime: 0,
                        lastStopTime: 0,
                        errorMessage: None,
                        cachedTools: None,
                        toolsCachedTime: 0,
                    },
                );
                changed = true;
            }
        }
        if changed {
            self.writeServerStatus(&status)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn sanitizeServerConfig(
        &self,
        serverId: &str,
        serverConfig: ServerConfig,
        _source: &str,
    ) -> Option<ServerConfig> {
        let command = serverConfig.command.trim().to_string();
        if command.is_empty() {
            return None;
        }
        Some(ServerConfig {
            command,
            args: serverConfig
                .args
                .into_iter()
                .filter(|item| !item.trim().is_empty())
                .collect(),
            disabled: serverConfig.disabled,
            autoApprove: serverConfig
                .autoApprove
                .into_iter()
                .filter(|item| !item.trim().is_empty())
                .collect(),
            env: cleanEnv(serverConfig.env),
        })
    }

    #[allow(non_snake_case)]
    fn sanitizeMCPConfig(&self, config: MCPConfig, source: &str) -> SanitizedConfigResult {
        let mut sanitizedServers = BTreeMap::new();
        let mut removedServerIds = Vec::new();
        for (serverId, serverConfig) in config.mcpServers {
            if let Some(sanitizedServer) =
                self.sanitizeServerConfig(&serverId, serverConfig, source)
            {
                sanitizedServers.insert(serverId, sanitizedServer);
            } else {
                removedServerIds.push(serverId);
            }
        }

        let mut sanitizedMetadata = config.pluginMetadata;
        let mut removedMetadataIds = Vec::new();
        for serverId in &removedServerIds {
            if sanitizedMetadata
                .get(serverId)
                .map(|metadata| metadata.r#type != "remote")
                .unwrap_or(false)
            {
                sanitizedMetadata.remove(serverId);
                removedMetadataIds.push(serverId.clone());
            }
        }

        SanitizedConfigResult {
            config: MCPConfig {
                mcpServers: sanitizedServers,
                pluginMetadata: sanitizedMetadata,
            },
            removedServerIds,
            removedMetadataIds,
        }
    }

    #[allow(non_snake_case)]
    fn autoFillMissingMetadata(&self, config: MCPConfig) -> MCPConfig {
        let mut metadata = config.pluginMetadata.clone();
        for serverId in config.mcpServers.keys() {
            if metadata.contains_key(serverId) {
                continue;
            }
            metadata.insert(
                serverId.clone(),
                PluginMetadata {
                    id: serverId.clone(),
                    name: displayNameFromId(serverId),
                    description: String::new(),
                    logoUrl: None,
                    author: "Unknown".to_string(),
                    isInstalled: true,
                    version: "1.0.0".to_string(),
                    updatedAt: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    longDescription: "Local auto detected MCP server".to_string(),
                    repoUrl: String::new(),
                    r#type: "local".to_string(),
                    endpoint: None,
                    connectionType: Some("httpStream".to_string()),
                    disabled: false,
                    bearerToken: None,
                    headers: None,
                    installedPath: None,
                    installedTime: currentTimeMillis(),
                    marketConfig: None,
                },
            );
        }
        MCPConfig {
            mcpServers: config.mcpServers,
            pluginMetadata: metadata,
        }
    }

    #[allow(non_snake_case)]
    fn readMCPConfig(&self) -> Result<MCPConfig, String> {
        let path = self.storePaths.mcp_config_path();
        if !path.exists() {
            return Ok(MCPConfig::default());
        }
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        serde_json::from_str::<MCPConfig>(&text).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn writeMCPConfig(&self, config: &MCPConfig) -> Result<(), String> {
        self.storePaths
            .ensure_mcp_plugins_dir()
            .map_err(|error| error.to_string())?;
        let text = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
        fs::write(self.storePaths.mcp_config_path(), text).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn readServerStatus(&self) -> Result<BTreeMap<String, ServerStatus>, String> {
        let path = self.storePaths.mcp_server_status_path();
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        serde_json::from_str::<BTreeMap<String, ServerStatus>>(&text)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn writeServerStatus(&self, status: &BTreeMap<String, ServerStatus>) -> Result<(), String> {
        self.storePaths
            .ensure_mcp_plugins_dir()
            .map_err(|error| error.to_string())?;
        let text = serde_json::to_string_pretty(status).map_err(|error| error.to_string())?;
        fs::write(self.storePaths.mcp_server_status_path(), text).map_err(|error| error.to_string())
    }
}

#[allow(non_snake_case)]
fn cleanEnv(env: BTreeMap<String, String>) -> BTreeMap<String, String> {
    env.into_iter()
        .filter(|(key, _)| !key.trim().is_empty())
        .collect()
}

#[allow(non_snake_case)]
fn displayNameFromId(serverId: &str) -> String {
    serverId
        .replace(['_', '-'], " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 { operit_host_api::TimeUtils::currentTimeMillis() }

#[allow(non_snake_case)]
fn unknownAuthor() -> String {
    "Unknown".to_string()
}

#[allow(non_snake_case)]
fn localType() -> String {
    "local".to_string()
}

#[allow(non_snake_case)]
fn httpStreamConnectionType() -> Option<String> {
    Some("httpStream".to_string())
}

#[allow(non_snake_case)]
fn emptyJsonObjectString() -> String {
    "{}".to_string()
}

