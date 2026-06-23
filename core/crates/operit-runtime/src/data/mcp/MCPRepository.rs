use std::fs;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::api::chat::enhance::MultiServiceManager::MultiServiceManager;
use crate::api::chat::llmprovider::AIService::{collect_stream_chunks, SendMessageRequest};
use operit_context::OperitApplicationContext::defaultHttpHost;
use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::core::config::FunctionalPrompts::FunctionalPrompts;
use crate::data::mcp::plugins::MCPBridgeClient::MCPBridgeClient;
use crate::data::mcp::plugins::MCPConfigGenerator::MCPConfigGenerator;
use crate::data::mcp::plugins::MCPProjectAnalyzer::MCPProjectAnalyzer;
use crate::data::mcp::MCPLocalServer::{MCPConfig, MCPLocalServer, PluginMetadata, ServerConfig};
use crate::data::model::FunctionType::FunctionType;
use crate::util::ChatUtils::ChatUtils;
use operit_host_api::HttpRequestData;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use url::Url;

const CONNECT_TIMEOUT_SECONDS: u64 = 15;
const READ_TIMEOUT_SECONDS: u64 = 30;

#[derive(Clone)]
pub struct MCPRepository {
    context: OperitApplicationContext,
    mcpLocalServer: MCPLocalServer,
    pluginsBaseDir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallResult {
    Success { pluginPath: String },
    Error { message: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallProgress {
    Preparing,
    Downloading(i32),
    Extracting(i32),
    Finished,
}

impl MCPRepository {
    #[allow(non_snake_case)]
    pub fn getInstance(context: &OperitApplicationContext) -> Self {
        let paths = RuntimeStorePaths::default();
        let _ = paths.ensure_mcp_plugins_dir();
        Self {
            context: context.clone(),
            mcpLocalServer: MCPLocalServer::getInstance(context),
            pluginsBaseDir: paths.mcp_plugins_dir(),
        }
    }

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

    #[allow(non_snake_case)]
    fn installPluginInternal(
        &self,
        pluginId: &str,
        repoUrl: &str,
        progressCallback: &impl Fn(InstallProgress),
    ) -> InstallResult {
        progressCallback(InstallProgress::Preparing);

        let pluginDir = self.pluginsBaseDir.join(pluginId);
        if pluginDir.exists() {
            let _ = fs::remove_dir_all(&pluginDir);
        }
        if let Err(error) = fs::create_dir_all(&pluginDir) {
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
        let Some(zipFile) =
            self.downloadRepositoryZip(&owner, &repoName, pluginId, progressCallback)
        else {
            return InstallResult::Error {
                message: "Failed to download repository zip".to_string(),
            };
        };

        progressCallback(InstallProgress::Extracting(0));
        if let Err(error) = extractZipFile(&zipFile, &pluginDir, progressCallback) {
            let _ = fs::remove_file(&zipFile);
            let _ = fs::remove_dir_all(&pluginDir);
            return InstallResult::Error {
                message: format!("Failed to extract repository: {error}"),
            };
        }
        let _ = fs::remove_file(&zipFile);

        let mainDir = fs::read_dir(&pluginDir)
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .find(|path| path.is_dir())
            })
            .unwrap_or(pluginDir);

        progressCallback(InstallProgress::Finished);
        InstallResult::Success {
            pluginPath: mainDir.to_string_lossy().to_string(),
        }
    }

    #[allow(non_snake_case)]
    fn installPluginFromZipInternal(
        &self,
        pluginId: &str,
        zipPath: &str,
        progressCallback: &impl Fn(InstallProgress),
    ) -> InstallResult {
        progressCallback(InstallProgress::Preparing);

        let zipFile = PathBuf::from(zipPath);
        if !zipFile.is_file() {
            return InstallResult::Error {
                message: format!("MCP zip file not found: {zipPath}"),
            };
        }
        if zipFile
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| !value.eq_ignore_ascii_case("zip"))
            .unwrap_or(true)
        {
            return InstallResult::Error {
                message: "Only .zip files are supported".to_string(),
            };
        }

        let pluginDir = self.pluginsBaseDir.join(pluginId);
        if pluginDir.exists() {
            let _ = fs::remove_dir_all(&pluginDir);
        }
        if let Err(error) = fs::create_dir_all(&pluginDir) {
            return InstallResult::Error {
                message: format!("Failed to create plugin directory: {error}"),
            };
        }

        progressCallback(InstallProgress::Extracting(0));
        if let Err(error) = extractZipFile(&zipFile, &pluginDir, progressCallback) {
            let _ = fs::remove_dir_all(&pluginDir);
            return InstallResult::Error {
                message: format!("Failed to extract MCP zip: {error}"),
            };
        }

        let mainDir = fs::read_dir(&pluginDir)
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .find(|path| path.is_dir())
            })
            .unwrap_or(pluginDir);

        progressCallback(InstallProgress::Finished);
        InstallResult::Success {
            pluginPath: mainDir.to_string_lossy().to_string(),
        }
    }

    #[allow(non_snake_case)]
    fn downloadRepositoryZip(
        &self,
        owner: &str,
        repoName: &str,
        serverId: &str,
        progressCallback: &impl Fn(InstallProgress),
    ) -> Option<PathBuf> {
        let defaultBranch = getGithubDefaultBranch(owner, repoName)?;
        let zipUrl = format!(
            "https://github.com/{owner}/{repoName}/archive/refs/heads/{}.zip",
            encodePathSegment(&defaultBranch)
        );
        downloadFromUrl(&zipUrl, serverId, progressCallback).ok()
    }

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

    #[allow(non_snake_case)]
    fn generateConfigFromProject(
        &self,
        pluginId: &str,
        pluginPath: &str,
    ) -> Result<String, String> {
        let pluginDir = PathBuf::from(pluginPath);
        if !pluginDir.is_dir() {
            return Err(format!("MCP plugin directory not found: {pluginPath}"));
        }
        let analyzer = MCPProjectAnalyzer;
        let readmeContent = analyzer
            .findReadmeFile(&pluginDir)
            .map(|path| fs::read_to_string(path).map_err(|error| error.to_string()))
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
        let generatedDescription =
            generatePackageDescription(&targetPluginName, &toolDescriptions).await?;
        if generatedDescription.trim().is_empty() {
            return Err("Generated description is empty".to_string());
        }
        Ok(generatedDescription)
    }

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
async fn generatePackageDescription(
    pluginName: &str,
    toolDescriptions: &[String],
) -> Result<String, String> {
    if toolDescriptions.is_empty() {
        return Ok(String::new());
    }
    let toolList = toolDescriptions
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    let useEnglish = false;
    let descriptionPrompt =
        FunctionalPrompts::packageDescriptionUserPrompt(pluginName, &toolList, useEnglish);
    let chatHistory = vec![
        PromptTurn::new(
            PromptTurnKind::SYSTEM,
            FunctionalPrompts::packageDescriptionSystemPrompt(useEnglish),
        ),
        PromptTurn::new(PromptTurnKind::USER, descriptionPrompt),
    ];
    let mut multiServiceManager = MultiServiceManager::default();
    multiServiceManager
        .initialize()
        .map_err(|error| error.to_string())?;
    let summaryService = multiServiceManager
        .getServiceForFunction(FunctionType::SUMMARY)
        .map_err(|error| error.to_string())?;
    let modelParameters = multiServiceManager
        .getModelParametersForFunction(FunctionType::SUMMARY)
        .map_err(|error| error.to_string())?;
    let mut service = summaryService.lock().await;
    let stream = service
        .send_message(SendMessageRequest {
            chat_history: chatHistory,
            model_parameters: modelParameters,
            enable_thinking: false,
            stream: true,
            available_tools: Vec::new(),
            preserve_think_in_history: false,
            enable_retry: true,
            on_non_fatal_error: None,
            on_tool_invocation: None,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(
        ChatUtils::remove_thinking_content(&collect_stream_chunks(stream).join(""))
            .trim()
            .to_string(),
    )
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
fn downloadFromUrl(
    zipUrl: &str,
    serverId: &str,
    progressCallback: &impl Fn(InstallProgress),
) -> Result<PathBuf, String> {
    let tempFile = std::env::temp_dir().join(format!(
        "operit_mcp_{}_repo_{}.zip",
        sanitizeTempPart(serverId),
        currentTimeMillis()
    ));
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
    fs::write(&tempFile, response.body).map_err(|error| error.to_string())?;
    progressCallback(InstallProgress::Downloading(100));
    Ok(tempFile)
}

#[allow(non_snake_case)]
fn extractZipFile(
    zipFile: &Path,
    targetDir: &Path,
    progressCallback: &impl Fn(InstallProgress),
) -> Result<(), String> {
    let file = fs::File::open(zipFile).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
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
        let outPath = targetDir.join(enclosedName);
        if entry.is_dir() {
            fs::create_dir_all(&outPath).map_err(|error| error.to_string())?;
        } else {
            if let Some(parent) = outPath.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut out = fs::File::create(&outPath).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|error| error.to_string())?;
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
fn sanitizeTempPart(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
