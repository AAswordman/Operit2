use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use zip::ZipArchive;

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::data::mcp::MCPLocalServer::{MCPConfig, MCPLocalServer, PluginMetadata};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

const CONNECT_TIMEOUT_SECONDS: u64 = 15;
const READ_TIMEOUT_SECONDS: u64 = 30;

#[derive(Clone, Debug)]
pub struct MCPRepository {
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
            mcpLocalServer: MCPLocalServer::getInstance(context),
            pluginsBaseDir: paths.mcp_plugins_dir(),
        }
    }

    #[allow(non_snake_case)]
    pub fn installMCPServerWithObject(
        &self,
        server: PluginMetadata,
        progressCallback: impl Fn(InstallProgress),
    ) -> InstallResult {
        let result = self.installPluginInternal(&server, &progressCallback);
        if let InstallResult::Success { pluginPath } = &result {
            if let Err(error) = self.savePluginMetadata(&server, pluginPath) {
                return InstallResult::Error { message: error };
            }
        }
        result
    }

    #[allow(non_snake_case)]
    pub fn checkConfigNeedsPhysicalInstallation(&self, jsonConfig: &str) -> bool {
        let Ok(config) = serde_json::from_str::<MCPConfig>(jsonConfig) else {
            return true;
        };
        if config.mcpServers.is_empty() {
            return true;
        }
        config
            .mcpServers
            .values()
            .any(|serverConfig| commandNeedsPhysicalInstallation(&serverConfig.command))
    }

    #[allow(non_snake_case)]
    fn installPluginInternal(
        &self,
        server: &PluginMetadata,
        progressCallback: &impl Fn(InstallProgress),
    ) -> InstallResult {
        progressCallback(InstallProgress::Preparing);

        let pluginDir = self.pluginsBaseDir.join(&server.id);
        if pluginDir.exists() {
            let _ = fs::remove_dir_all(&pluginDir);
        }
        if let Err(error) = fs::create_dir_all(&pluginDir) {
            return InstallResult::Error {
                message: format!("Failed to create plugin directory: {error}"),
            };
        }

        let Some((owner, repoName)) = extractOwnerAndRepo(&server.repoUrl) else {
            return InstallResult::Error {
                message: "Invalid GitHub repository URL".to_string(),
            };
        };

        progressCallback(InstallProgress::Downloading(0));
        let Some(zipFile) = self.downloadRepositoryZip(&owner, &repoName, &server.id, progressCallback) else {
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
    fn savePluginMetadata(&self, server: &PluginMetadata, pluginPath: &str) -> Result<(), String> {
        let mut metadata = server.clone();
        metadata.r#type = "local".to_string();
        metadata.installedPath = Some(pluginPath.to_string());
        metadata.installedTime = currentTimeMillis();
        self.mcpLocalServer.addOrUpdatePluginMetadata(metadata)
    }
}

#[allow(non_snake_case)]
fn commandNeedsPhysicalInstallation(command: &str) -> bool {
    !matches!(command.trim().to_ascii_lowercase().as_str(), "npx" | "uvx" | "uv")
}

#[allow(non_snake_case)]
fn extractOwnerAndRepo(repoUrl: &str) -> Option<(String, String)> {
    let normalized = repoUrl.trim().trim_end_matches(".git");
    let url = if normalized.starts_with("http://") || normalized.starts_with("https://") {
        reqwest::Url::parse(normalized).ok()?
    } else {
        reqwest::Url::parse(&format!("https://{normalized}")).ok()?
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
fn getGithubDefaultBranch(owner: &str, repoName: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repoName}");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .user_agent("Operit-Market")
        .build()
        .ok()?;
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let value = response.json::<serde_json::Value>().ok()?;
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
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(READ_TIMEOUT_SECONDS))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|error| error.to_string())?;
    let mut response = client.get(zipUrl).send().map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let contentLength = response.content_length().unwrap_or(0);
    let mut out = fs::File::create(&tempFile).map_err(|error| error.to_string())?;
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = std::io::Read::read(&mut response, &mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut out, &buffer[..read]).map_err(|error| error.to_string())?;
        downloaded += read as u64;
        let progress = if contentLength > 0 {
            ((downloaded * 100) / contentLength) as i32
        } else {
            -1
        };
        progressCallback(InstallProgress::Downloading(progress));
    }
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
        progressCallback(InstallProgress::Extracting(((index + 1) * 100 / totalEntries) as i32));
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
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}
