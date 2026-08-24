use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use zip::ZipArchive;

use crate::runtime_support::ToolRuntimeSupport;
use crate::tools::mcp_runtime::plugins::MCPBridgeClient::MCPBridgeClient;
use crate::tools::mcp_runtime::plugins::MCPConfigGenerator::MCPConfigGenerator;
use crate::tools::mcp_runtime::plugins::MCPProjectAnalyzer::MCPProjectAnalyzer;
use crate::tools::mcp_runtime::MCPLocalServer::{
    MCPConfig, MCPLocalServer, PluginMetadata, ServerConfig,
};
use operit_host_api::{
    FileSystemHost, HostManager::defaultHttpHost, HostManager::HostManager, HttpRequestData,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use url::Url;

const CONNECT_TIMEOUT_SECONDS: u64 = 15;
const READ_TIMEOUT_SECONDS: u64 = 30;

/// Repository for installing MCP plugins and deriving plugin metadata.
#[derive(Clone)]
pub struct MCPRepository {
    context: HostManager,
    mcpLocalServer: MCPLocalServer,
    fileSystemHost: Arc<dyn FileSystemHost>,
    pluginsBaseDir: String,
    runtimeSupport: Arc<dyn ToolRuntimeSupport>,
}

/// Result of an MCP plugin install request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallResult {
    /// Plugin installed successfully and resolved to a local path.
    Success { pluginPath: String },
    /// Plugin installation failed with a user-facing message.
    Error { message: String },
}

/// Progress events emitted while installing an MCP plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallProgress {
    /// Preparing directories and input state.
    Preparing,
    /// Downloading a repository archive.
    Downloading(i32),
    /// Extracting a plugin archive.
    Extracting(i32),
    /// Installation completed.
    Finished,
}

impl MCPRepository {
    /// Creates an MCP repository from the current application context.
    #[allow(non_snake_case)]
    pub fn getInstance(context: &HostManager, runtimeSupport: Arc<dyn ToolRuntimeSupport>) -> Self {
        let paths = RuntimeStorePaths::default();
        let fileSystemHost = context
            .fileSystemHost
            .clone()
            .expect("MCPRepository requires a FileSystemHost");
        let pluginsBaseDir = paths.mcp_plugins_dir().to_string_lossy().to_string();
        fileSystemHost
            .makeDirectory(&pluginsBaseDir, true)
            .expect("MCPRepository cannot create the MCP plugin directory");
        Self {
            context: context.clone(),
            mcpLocalServer: MCPLocalServer::getInstance(context),
            fileSystemHost,
            pluginsBaseDir,
            runtimeSupport,
        }
    }

    /// Installs an MCP server from a repository URL and metadata object.
    #[allow(non_snake_case)]
    pub fn installMCPServerWithObject(
        &self,
        pluginId: String,
        repoUrl: String,
        server: PluginMetadata,
        mcpConfig: String,
        progressCallback: impl Fn(InstallProgress),
    ) -> InstallResult {
        let result = self.installPluginInternal(&pluginId, &repoUrl, &progressCallback);
        if let InstallResult::Success { pluginPath } = &result {
            if let Err(error) =
                self.deployInstalledPlugin(&pluginId, pluginPath, &server, &mcpConfig)
            {
                return InstallResult::Error { message: error };
            }
        }
        result
    }

    /// Installs an MCP server from a local zip and metadata object.
    #[allow(non_snake_case)]
    pub fn installMCPServerFromZip(
        &self,
        pluginId: String,
        zipPath: String,
        server: PluginMetadata,
        mcpConfig: String,
        progressCallback: impl Fn(InstallProgress),
    ) -> InstallResult {
        let result = self.installPluginFromZipInternal(&pluginId, &zipPath, &progressCallback);
        if let InstallResult::Success { pluginPath } = &result {
            if let Err(error) =
                self.deployInstalledPlugin(&pluginId, pluginPath, &server, &mcpConfig)
            {
                return InstallResult::Error { message: error };
            }
        }
        result
    }

    /// Installs an MCP server from a repository URL for Flutter bridge callers.
    #[allow(non_snake_case)]
    pub fn installMCPServerWithObjectForFlutter(
        &self,
        pluginId: String,
        repoUrl: String,
        name: String,
        description: String,
        mcpConfig: String,
    ) -> Result<String, String> {
        let server = PluginMetadata {
            name,
            description,
            author: String::new(),
            version: String::new(),
        };
        match self.installMCPServerWithObject(pluginId, repoUrl, server, mcpConfig, |_| {}) {
            InstallResult::Success { pluginPath } => Ok(pluginPath),
            InstallResult::Error { message } => Err(message),
        }
    }

    /// Installs an MCP server from a local zip for Flutter bridge callers.
    #[allow(non_snake_case)]
    pub fn installMCPServerFromZipForFlutter(
        &self,
        pluginId: String,
        zipPath: String,
        name: String,
        description: String,
        mcpConfig: String,
    ) -> Result<String, String> {
        let server = PluginMetadata {
            name,
            description,
            author: String::new(),
            version: String::new(),
        };
        match self.installMCPServerFromZip(pluginId, zipPath, server, mcpConfig, |_| {}) {
            InstallResult::Success { pluginPath } => Ok(pluginPath),
            InstallResult::Error { message } => Err(message),
        }
    }

    /// Downloads and extracts a GitHub repository into the local plugin directory.
    #[allow(non_snake_case)]
    fn installPluginInternal(
        &self,
        pluginId: &str,
        repoUrl: &str,
        progressCallback: &impl Fn(InstallProgress),
    ) -> InstallResult {
        progressCallback(InstallProgress::Preparing);

        let pluginDir = joinHostPath(&self.pluginsBaseDir, pluginId);
        if let Err(error) = self.fileSystemHost.deleteFile(&pluginDir, true) {
            return InstallResult::Error {
                message: format!("Failed to reset plugin directory: {error}"),
            };
        }
        if let Err(error) = self.fileSystemHost.makeDirectory(&pluginDir, true) {
            return InstallResult::Error {
                message: format!("Failed to create plugin directory: {error}"),
            };
        }

        let Some((owner, repoName)) = extractOwnerAndRepo(repoUrl) else {
            return InstallResult::Error {
                message: "Invalid GitHub repository URL".to_string(),
            };
        };

        progressCallback(InstallProgress::Downloading(0));
        let Some(zipBytes) =
            self.downloadRepositoryZip(&owner, &repoName, pluginId, progressCallback)
        else {
            return InstallResult::Error {
                message: "Failed to download repository zip".to_string(),
            };
        };

        progressCallback(InstallProgress::Extracting(0));
        if let Err(error) = extractZipBytes(
            &zipBytes,
            &pluginDir,
            self.fileSystemHost.as_ref(),
            progressCallback,
        ) {
            let _ = self.fileSystemHost.deleteFile(&pluginDir, true);
            return InstallResult::Error {
                message: format!("Failed to extract repository: {error}"),
            };
        }
        let mainDir = match self.resolvePluginRoot(&pluginDir) {
            Ok(path) => path,
            Err(error) => {
                let _ = self.fileSystemHost.deleteFile(&pluginDir, true);
                return InstallResult::Error { message: error };
            }
        };

        progressCallback(InstallProgress::Finished);
        InstallResult::Success {
            pluginPath: mainDir,
        }
    }

    /// Extracts a local MCP zip into the local plugin directory.
    #[allow(non_snake_case)]
    fn installPluginFromZipInternal(
        &self,
        pluginId: &str,
        zipPath: &str,
        progressCallback: &impl Fn(InstallProgress),
    ) -> InstallResult {
        progressCallback(InstallProgress::Preparing);

        let zipInfo = match self.fileSystemHost.fileExists(zipPath) {
            Ok(info) => info,
            Err(error) => {
                return InstallResult::Error {
                    message: format!("Failed to access MCP zip: {error}"),
                }
            }
        };
        if !zipInfo.exists || zipInfo.isDirectory {
            return InstallResult::Error {
                message: format!("MCP zip file not found: {zipPath}"),
            };
        }
        if !zipPath.trim().to_ascii_lowercase().ends_with(".zip") {
            return InstallResult::Error {
                message: "Only .zip files are supported".to_string(),
            };
        }

        let pluginDir = joinHostPath(&self.pluginsBaseDir, pluginId);
        if let Err(error) = self.fileSystemHost.deleteFile(&pluginDir, true) {
            return InstallResult::Error {
                message: format!("Failed to reset plugin directory: {error}"),
            };
        }
        if let Err(error) = self.fileSystemHost.makeDirectory(&pluginDir, true) {
            return InstallResult::Error {
                message: format!("Failed to create plugin directory: {error}"),
            };
        }

        progressCallback(InstallProgress::Extracting(0));
        let zipBytes = match self.fileSystemHost.readFileBytes(zipPath) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = self.fileSystemHost.deleteFile(&pluginDir, true);
                return InstallResult::Error {
                    message: format!("Failed to read MCP zip: {error}"),
                };
            }
        };
        if let Err(error) = extractZipBytes(
            &zipBytes,
            &pluginDir,
            self.fileSystemHost.as_ref(),
            progressCallback,
        ) {
            let _ = self.fileSystemHost.deleteFile(&pluginDir, true);
            return InstallResult::Error {
                message: format!("Failed to extract MCP zip: {error}"),
            };
        }

        let mainDir = match self.resolvePluginRoot(&pluginDir) {
            Ok(path) => path,
            Err(error) => {
                let _ = self.fileSystemHost.deleteFile(&pluginDir, true);
                return InstallResult::Error { message: error };
            }
        };

        progressCallback(InstallProgress::Finished);
        InstallResult::Success {
            pluginPath: mainDir,
        }
    }

    /// Downloads the default branch archive for a GitHub repository.
    #[allow(non_snake_case)]
    fn downloadRepositoryZip(
        &self,
        owner: &str,
        repoName: &str,
        _serverId: &str,
        progressCallback: &impl Fn(InstallProgress),
    ) -> Option<Vec<u8>> {
        let defaultBranch = getGithubDefaultBranch(owner, repoName)?;
        let zipUrl = format!(
            "https://github.com/{owner}/{repoName}/archive/refs/heads/{}.zip",
            encodePathSegment(&defaultBranch)
        );
        downloadArchiveBytes(&zipUrl, progressCallback).ok()
    }

    /// Resolves the repository root after an archive has been extracted.
    #[allow(non_snake_case)]
    fn resolvePluginRoot(&self, pluginDir: &str) -> Result<String, String> {
        let roots = self
            .fileSystemHost
            .listFiles(pluginDir)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|entry| entry.isDirectory)
            .collect::<Vec<_>>();
        if roots.len() != 1 {
            return Err(format!(
                "MCP archive must contain exactly one root directory: {pluginDir}"
            ));
        }
        Ok(joinHostPath(pluginDir, &roots[0].name))
    }

    /// Stores plugin metadata in the local MCP registry.
    #[allow(non_snake_case)]
    fn savePluginMetadata(
        &self,
        pluginId: &str,
        server: &PluginMetadata,
        _pluginPath: &str,
    ) -> Result<(), String> {
        self.mcpLocalServer
            .addOrUpdatePluginMetadata(pluginId, server.clone())
    }

    /// Writes MCP server config and metadata for an installed plugin.
    #[allow(non_snake_case)]
    fn deployInstalledPlugin(
        &self,
        pluginId: &str,
        pluginPath: &str,
        server: &PluginMetadata,
        mcpConfig: &str,
    ) -> Result<(), String> {
        let configJson = if mcpConfig.trim().is_empty() {
            self.generateConfigFromProject(pluginId, pluginPath)?
        } else {
            mcpConfig.to_string()
        };
        let serverConfig = firstServerConfigFromJson(&configJson)?;
        self.mcpLocalServer
            .addOrUpdateMCPServerConfig(pluginId.to_string(), serverConfig)?;
        self.savePluginMetadata(pluginId, server, pluginPath)?;
        self.mcpLocalServer.reloadConfigurations()
    }

    /// Generates MCP config from a plugin project directory.
    #[allow(non_snake_case)]
    fn generateConfigFromProject(
        &self,
        pluginId: &str,
        pluginPath: &str,
    ) -> Result<String, String> {
        let pluginDir = PathBuf::from(pluginPath);
        let pluginInfo = self
            .fileSystemHost
            .fileExists(pluginPath)
            .map_err(|error| error.to_string())?;
        if !pluginInfo.exists || !pluginInfo.isDirectory {
            return Err(format!("MCP plugin directory not found: {pluginPath}"));
        }
        let analyzer = MCPProjectAnalyzer::new(self.fileSystemHost.clone());
        let readmeContent = analyzer
            .findReadmeFile(&pluginDir)
            .map(|path| {
                self.fileSystemHost
                    .readFile(
                        path.to_str()
                            .ok_or_else(|| "MCP README path is not valid UTF-8".to_string())?,
                    )
                    .map_err(|error| error.to_string())
            })
            .transpose()?
            .unwrap_or_default();
        let projectStructure = analyzer.analyzeProjectStructure(&pluginDir, &readmeContent);
        let configGenerator = MCPConfigGenerator;
        Ok(configGenerator.generateMcpConfig(
            pluginId,
            &projectStructure,
            Default::default(),
            Some(&self.mcpLocalServer.getPluginRuntimeDirectory(pluginId)),
        ))
    }

    /// Generates a concise plugin description from available MCP tool descriptions.
    #[allow(non_snake_case)]
    pub async fn generatePluginDescription(
        &self,
        pluginId: &str,
        pluginName: &str,
    ) -> Result<String, String> {
        let metadata = self
            .mcpLocalServer
            .getPluginMetadata(pluginId)
            .ok_or_else(|| "MCP server not found".to_string())?;
        let toolDescriptions = self.collectToolDescriptionsForDescriptionGeneration(pluginId);
        if toolDescriptions.is_empty() {
            return Err("No tools available for description generation".to_string());
        }
        let targetPluginName = if pluginName.trim().is_empty() {
            metadata.name
        } else {
            pluginName.trim().to_string()
        };
        let generatedDescription = self
            .runtimeSupport
            .generateMcpPluginDescription(&targetPluginName, &toolDescriptions)
            .await?;
        if generatedDescription.trim().is_empty() {
            return Err("Generated description is empty".to_string());
        }
        Ok(generatedDescription)
    }

    /// Collects tool descriptions from cached metadata or the live MCP bridge.
    #[allow(non_snake_case)]
    fn collectToolDescriptionsForDescriptionGeneration(&self, pluginId: &str) -> Vec<String> {
        let cachedToolDescriptions = self
            .mcpLocalServer
            .getCachedTools(pluginId)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|cachedTool| {
                let toolName = cachedTool.name.trim().to_string();
                if toolName.is_empty() {
                    return None;
                }
                let description = cachedTool.description.trim().to_string();
                if description.is_empty() {
                    Some(toolName)
                } else {
                    Some(format!("{toolName}: {description}"))
                }
            })
            .collect::<Vec<_>>();
        if !cachedToolDescriptions.is_empty() {
            return cachedToolDescriptions;
        }

        let serviceName = self.serviceNameForDescriptionGeneration(pluginId);
        MCPBridgeClient::new(self.context.clone(), serviceName).getToolDescriptions()
    }

    /// Resolves the service name used when querying live MCP tool descriptions.
    #[allow(non_snake_case)]
    fn serviceNameForDescriptionGeneration(&self, pluginId: &str) -> String {
        let pluginConfig = self.mcpLocalServer.getPluginConfig(pluginId);
        extractServerNameFromConfig(&pluginConfig).unwrap_or_else(|| {
            pluginId
                .split('/')
                .last()
                .unwrap_or(pluginId)
                .to_ascii_lowercase()
        })
    }
}

#[allow(non_snake_case)]
fn extractOwnerAndRepo(repoUrl: &str) -> Option<(String, String)> {
    let normalized = repoUrl.trim().trim_end_matches(".git");
    let url = if normalized.starts_with("http://") || normalized.starts_with("https://") {
        Url::parse(normalized).ok()?
    } else {
        Url::parse(&format!("https://{normalized}")).ok()?
    };
    let host = url.host_str()?.to_ascii_lowercase();
    if host != "github.com" && !host.ends_with(".github.com") {
        return None;
    }
    let segments = url
        .path_segments()
        .map(|segments| segments.filter(|item| !item.is_empty()).collect::<Vec<_>>())?;
    if segments.len() < 2 {
        return None;
    }
    let owner = segments[0].to_string();
    let repo = segments[1].trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        None
    } else {
        Some((owner, repo))
    }
}

#[allow(non_snake_case)]
fn firstServerConfigFromJson(configJson: &str) -> Result<ServerConfig, String> {
    let config =
        serde_json::from_str::<MCPConfig>(configJson).map_err(|error| error.to_string())?;
    config
        .mcpServers
        .into_values()
        .next()
        .ok_or_else(|| "MCP config has no mcpServers entry".to_string())
}

#[allow(non_snake_case)]
fn extractServerNameFromConfig(configJson: &str) -> Option<String> {
    if configJson.trim().is_empty() {
        return None;
    }
    let value = serde_json::from_str::<serde_json::Value>(configJson).ok()?;
    value
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)?
        .keys()
        .next()
        .cloned()
}

#[allow(non_snake_case)]
fn getGithubDefaultBranch(owner: &str, repoName: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repoName}");
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url,
            method: "GET".to_string(),
            headers: vec![
                (
                    "Accept".to_string(),
                    "application/vnd.github.v3+json".to_string(),
                ),
                ("User-Agent".to_string(), "Operit-Market".to_string()),
            ],
            body: Vec::new(),
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: CONNECT_TIMEOUT_SECONDS,
            readTimeoutSeconds: CONNECT_TIMEOUT_SECONDS,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        })
        .ok()?;
    if !(200..300).contains(&response.statusCode) {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(&response.body).ok()?;
    value
        .get("default_branch")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

#[allow(non_snake_case)]
fn downloadArchiveBytes(
    zipUrl: &str,
    progressCallback: &impl Fn(InstallProgress),
) -> Result<Vec<u8>, String> {
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url: zipUrl.to_string(),
            method: "GET".to_string(),
            headers: vec![(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
            )],
            body: Vec::new(),
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: READ_TIMEOUT_SECONDS,
            readTimeoutSeconds: READ_TIMEOUT_SECONDS,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        })
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&response.statusCode) {
        return Err(format!("HTTP {}", response.statusCode));
    }
    progressCallback(InstallProgress::Downloading(100));
    Ok(response.body)
}

#[allow(non_snake_case)]
fn extractZipBytes(
    zipBytes: &[u8],
    targetDir: &str,
    fileSystemHost: &dyn FileSystemHost,
    progressCallback: &impl Fn(InstallProgress),
) -> Result<(), String> {
    let mut archive = ZipArchive::new(Cursor::new(zipBytes)).map_err(|error| error.to_string())?;
    let totalEntries = archive.len().max(1);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let entryName = entry.name().replace('\\', "/");
        if entryName.contains("__MACOSX") || entryName.ends_with(".DS_Store") {
            continue;
        }
        let Some(enclosedName) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };
        let outPath = joinHostPath(targetDir, &enclosedName.to_string_lossy());
        if entry.is_dir() {
            fileSystemHost
                .makeDirectory(&outPath, true)
                .map_err(|error| error.to_string())?;
        } else {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            fileSystemHost
                .writeFileBytes(&outPath, &bytes)
                .map_err(|error| error.to_string())?;
        }
        progressCallback(InstallProgress::Extracting(
            ((index + 1) * 100 / totalEntries) as i32,
        ));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn encodePathSegment(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            out.push(ch);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[allow(non_snake_case)]
fn joinHostPath(directory: &str, relativePath: &str) -> String {
    format!(
        "{}/{}",
        directory.trim_end_matches(['/', '\\']),
        relativePath
    )
}
