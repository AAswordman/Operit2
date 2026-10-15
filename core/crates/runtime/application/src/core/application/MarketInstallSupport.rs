//! Marketplace installation markers are owned by Core, so remote Flutter clients
//! observe the same installed versions as the runtime that owns the plugins.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use operit_host_api::FileSystemHost;
use operit_providers::market::MarketStatsApiService::{MarketEntrySummary, MarketStatsApiService};
use operit_store::ExtensionStore::ExtensionStore;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_util::AppLogger::AppLogger;
use operit_util::RuntimeStorageLayout::EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::OperitApplication::OperitApplication;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarketInstallMarker {
    entry_id: String,
    version_id: String,
}

fn marker_path(root: &Path) -> PathBuf {
    root.join(".operit").join("market.json")
}

fn read_marker(fs: &dyn FileSystemHost, root: &Path) -> Option<MarketInstallMarker> {
    let path = marker_path(root).to_string_lossy().into_owned();
    // Missing, corrupt, or inaccessible markers must not break the entire market.
    let text = fs.readFile(&path).ok()?;
    let marker: MarketInstallMarker = serde_json::from_str(&text).ok()?;
    if marker.entry_id.trim().is_empty() || marker.version_id.trim().is_empty() {
        return None;
    }
    Some(marker)
}

fn write_marker(
    fs: &dyn FileSystemHost,
    root: &Path,
    entry_id: &str,
    version_id: &str,
) -> Result<(), String> {
    if entry_id.trim().is_empty() || version_id.trim().is_empty() {
        return Err("Market entry/version id is missing".into());
    }
    let path = marker_path(root);
    fs.makeDirectory(&path.parent().unwrap().to_string_lossy(), true)
        .map_err(|error| error.to_string())?;
    let marker = MarketInstallMarker {
        entry_id: entry_id.into(),
        version_id: version_id.into(),
    };
    fs.writeFile(
        &path.to_string_lossy(),
        &serde_json::to_string(&marker).map_err(|error| error.to_string())?,
        false,
    )
    .map_err(|error| error.to_string())
}

fn artifact_marker_root(package_name: &str) -> Result<PathBuf, String> {
    let config_path = ExtensionStore::default()
        .configPath(package_name)
        .or_else(|_| ExtensionStore::configPathForScope(package_name, "device"))?;
    Ok(RuntimeStorePaths::default().runtime_storage_path(&config_path))
}

fn mcp_config_marker_root(server_id: &str) -> PathBuf {
    RuntimeStorePaths::default()
        .runtime_storage_path(EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH)
        .join(format!(
            "market-mcp-{:x}",
            Sha256::digest(server_id.as_bytes())
        ))
}

fn installed_marker_roots(app: &OperitApplication) -> Result<Vec<PathBuf>, String> {
    let fs = app
        .hostManager
        .fileSystemHost
        .as_ref()
        .ok_or("FileSystemHost is missing")?;
    let mut roots = Vec::new();
    let packages = app.packageManager();
    for source in packages
        .lock()
        .map_err(|error| error.to_string())?
        .getPublishablePackageSources()
    {
        roots.push(artifact_marker_root(&source.packageName)?);
    }
    for skill in app
        .skillRepository()
        .getAvailableSkillPackages()
        .into_values()
    {
        roots.push(skill.directory);
    }
    let mcp = MCPLocalServer::getInstance(&app.hostManager);
    for id in mcp.getAllMCPServersChecked()?.keys() {
        roots.push(mcp_config_marker_root(id));
    }
    let plugins = RuntimeStorePaths::default().mcp_plugins_dir();
    if let Ok(entries) = fs.listFiles(&plugins.to_string_lossy()) {
        for entry in entries.into_iter().filter(|entry| entry.isDirectory) {
            roots.push(plugins.join(entry.name));
        }
    }
    Ok(roots)
}

pub(super) fn installed_versions(
    app: &OperitApplication,
) -> Result<BTreeMap<String, String>, String> {
    let fs = app
        .hostManager
        .fileSystemHost
        .as_ref()
        .ok_or("FileSystemHost is missing")?;
    Ok(installed_marker_roots(app)?
        .into_iter()
        .filter_map(|root| read_marker(fs.as_ref(), &root))
        .map(|marker| (marker.entry_id, marker.version_id))
        .collect())
}

/// Imports a market entry and records only the version that actually succeeded.
pub(super) fn install_entry(
    app: &OperitApplication,
    entry_id: &str,
    requested_version_id: Option<&str>,
) -> Result<String, String> {
    let entry = MarketStatsApiService::new().get_entry_by_id(entry_id)?;
    let version_id = requested_version_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .or_else(|| {
            entry
                .latest_version
                .as_ref()
                .map(|version| version.id.trim())
        })
        .filter(|id| !id.is_empty())
        .ok_or("Marketplace entry has no installable version")?;
    let fs = app
        .hostManager
        .fileSystemHost
        .as_ref()
        .ok_or("FileSystemHost is missing")?;
    match entry.r#type.as_str() {
        "script" | "package" => {
            let asset = entry
                .assets
                .iter()
                .find(|asset| asset.version_id == version_id && !asset.id.trim().is_empty())
                .ok_or("Marketplace entry has no downloadable asset for version")?;
            let file_name = asset
                .asset_name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .ok_or("Marketplace asset has no file name")?;
            let result =
                app.installMarketArtifact(asset.id.clone(), file_name.into(), asset.sha256.clone());
            let result = result?;
            if !result
                .to_ascii_lowercase()
                .starts_with("successfully imported")
            {
                return Err(result);
            }
            let runtime_id = result
                .lines()
                .next()
                .and_then(|line| line.split_once(": "))
                .map(|(_, id)| id.trim())
                .filter(|id| !id.is_empty())
                .ok_or("Installed artifact package id is missing")?;
            write_marker(
                fs.as_ref(),
                &artifact_marker_root(runtime_id)?,
                &entry.id,
                version_id,
            )?;
        }
        "skill" => {
            install_skill(app, &entry, version_id)?;
        }
        "mcp" => {
            let config = entry
                .repo_version
                .as_ref()
                .and_then(|version| version.install_config.as_deref())
                .or_else(|| {
                    entry
                        .latest_version
                        .as_ref()
                        .and_then(|version| version.install_config.as_deref())
                })
                .unwrap_or_default();
            let repo_url = entry
                .source
                .as_ref()
                .map(|source| source.url.trim())
                .unwrap_or_default();
            if repo_url.is_empty() {
                let mcp = MCPLocalServer::getInstance(&app.hostManager);
                let parsed: serde_json::Value =
                    serde_json::from_str(config).map_err(|error| error.to_string())?;
                let servers = parsed
                    .get("mcpServers")
                    .and_then(|value| value.as_object())
                    .ok_or("MCP configuration is missing mcpServers")?;
                if mcp.mergeConfigFromJson(config)? == 0 {
                    return Err("MCP configuration contains no installable servers".into());
                }
                for id in servers.keys() {
                    write_marker(
                        fs.as_ref(),
                        &mcp_config_marker_root(id),
                        &entry.id,
                        version_id,
                    )?;
                }
            } else {
                let plugin_id = safe_package_id(&entry.title);
                let root = RuntimeStorePaths::default()
                    .mcp_plugins_dir()
                    .join(&plugin_id);
                let backup = stage_existing_directory(fs.as_ref(), &root)?;
                let result = (|| {
                    app.mcpRepository().installMCPServerWithObjectForFlutter(
                        plugin_id.clone(),
                        repo_url.into(),
                        entry.title.clone(),
                        entry.description.clone(),
                        config.into(),
                    )?;
                    write_marker(fs.as_ref(), &root, &entry.id, version_id)
                })();
                finish_directory_update(fs.as_ref(), &root, backup.as_deref(), result)?;
            }
        }
        other => return Err(format!("Unsupported marketplace entry type: {other}")),
    }
    Ok(version_id.into())
}

fn install_skill(
    app: &OperitApplication,
    entry: &MarketEntrySummary,
    version_id: &str,
) -> Result<(), String> {
    let repo_url = entry
        .source
        .as_ref()
        .map(|source| source.url.trim())
        .filter(|url| !url.is_empty())
        .ok_or("Skill repository URL is missing")?;
    let fs = app
        .hostManager
        .fileSystemHost
        .as_ref()
        .ok_or("FileSystemHost is missing")?;
    let repo = app.skillRepository();
    let old_root = repo
        .getAvailableSkillPackages()
        .into_values()
        .find_map(|skill| {
            read_marker(fs.as_ref(), &skill.directory)
                .filter(|marker| marker.entry_id == entry.id)
                .map(|_| skill.directory)
        });
    // Skill imports reject existing directories. Stage a market-owned installation
    // outside the scanned skills tree, restoring it if downloading/importing fails.
    let backup = match old_root.as_ref() {
        Some(root) => stage_existing_directory(fs.as_ref(), root)?,
        None => None,
    };
    let result = repo.importSkillFromGitHubRepo(repo_url);
    let imported = result
        .strip_prefix("Imported skill: ")
        .map(|value| value.split(" - ").next().unwrap_or(value).trim());
    if let Some(name) = imported {
        let root = RuntimeStorePaths::default().skills_dir().join(name);
        let result = write_marker(fs.as_ref(), &root, &entry.id, version_id);
        if result.is_err() {
            let _ = fs.deleteFile(&root.to_string_lossy(), true);
        }
        finish_directory_update(
            fs.as_ref(),
            old_root.as_deref().unwrap_or(&root),
            backup.as_deref(),
            result,
        )
    } else {
        match old_root.as_deref() {
            Some(root) => {
                finish_directory_update(fs.as_ref(), root, backup.as_deref(), Err(result))
            }
            None => Err(result),
        }
    }
}

/// Preserve the old plugin and marker until the replacement is fully installed.
fn stage_existing_directory(
    fs: &dyn FileSystemHost,
    root: &Path,
) -> Result<Option<PathBuf>, String> {
    if !fs
        .fileExists(&root.to_string_lossy())
        .map_err(|error| error.to_string())?
        .exists
    {
        return Ok(None);
    }
    let backup = RuntimeStorePaths::default()
        .toolpkg_cache_dir()
        .join(format!("market-update-{}", uuid::Uuid::new_v4()));
    fs.makeDirectory(&backup.parent().unwrap().to_string_lossy(), true)
        .map_err(|error| error.to_string())?;
    fs.moveFile(&root.to_string_lossy(), &backup.to_string_lossy())
        .map_err(|error| error.to_string())?;
    Ok(Some(backup))
}

fn finish_directory_update(
    fs: &dyn FileSystemHost,
    root: &Path,
    backup: Option<&Path>,
    result: Result<(), String>,
) -> Result<(), String> {
    if let Some(backup) = backup {
        if let Err(error) = result {
            // The new import may have left a partially extracted directory behind.
            let _ = fs.deleteFile(&root.to_string_lossy(), true);
            fs.moveFile(&backup.to_string_lossy(), &root.to_string_lossy())
                .map_err(|restore_error| {
                    format!("{error}; failed to restore previous plugin: {restore_error}")
                })?;
            return Err(error);
        }
        if let Err(error) = fs.deleteFile(&backup.to_string_lossy(), true) {
            AppLogger::w(
                "MarketInstall",
                &format!("Failed to delete update backup: {error}"),
            );
        }
    }
    result
}

fn safe_package_id(raw: &str) -> String {
    let mut id = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch);
        } else if !id.ends_with('_') {
            id.push('_');
        }
    }
    let id = id.trim_matches('_');
    if id.is_empty() {
        "market_item".into()
    } else {
        id.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_matches_kotlin_wire_format() {
        let marker: MarketInstallMarker =
            serde_json::from_str(r#"{"entryId":"entry","versionId":"v1"}"#).unwrap();
        assert_eq!(marker.entry_id, "entry");
        assert_eq!(marker.version_id, "v1");
        assert_eq!(
            marker_path(Path::new("plugin")),
            Path::new("plugin/.operit/market.json")
        );
    }

    /// Initializes the required storage roots before verifying that untrusted server IDs remain one path component.
    #[test]
    fn config_marker_names_do_not_trust_server_ids_as_paths() {
        let testRoot =
            std::env::temp_dir().join(format!("operit-market-marker-{}", uuid::Uuid::new_v4()));
        operit_util::RuntimeStoreRoot::setDefaultRuntimeStoreRootConfig(
            operit_util::RuntimeStoreRoot::RuntimeStoreRootConfig::new(
                testRoot.join("runtime"),
                testRoot.join("workspaces"),
            ),
        );
        let root = mcp_config_marker_root("../../outside");
        assert_eq!(
            root.parent().unwrap(),
            RuntimeStorePaths::default().runtime_storage_path(EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH)
        );
        assert!(root
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("market-mcp-"));
    }
}
