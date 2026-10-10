use std::path::PathBuf;

use crate::RuntimeStorageLayout as Layout;
use crate::RuntimeStoreRoot::{default_runtime_dir, default_workspace_dir};

#[derive(Debug, Clone, Default)]
pub struct OperitPaths;

pub const CONFIG_PREFERENCES_DIR_PATH: &str = Layout::CONFIG_PREFERENCES_DIR_PATH;

pub const RUNTIME_USER_ASSETS_DIR_PATH: &str = Layout::RUNTIME_USER_ASSETS_DIR_PATH;
pub const RUNTIME_THEME_ASSETS_DIR_PATH: &str = Layout::RUNTIME_THEME_ASSETS_DIR_PATH;
pub const RUNTIME_SHARE_IMAGE_DIR_PATH: &str = Layout::RUNTIME_SHARE_IMAGE_DIR_PATH;
pub const RUNTIME_WORKSPACE_VIDEO_DIR_PATH: &str = Layout::RUNTIME_WORKSPACE_VIDEO_DIR_PATH;
pub const RUNTIME_COMPOSE_DSL_WEBVIEW_FILES_DIR_PATH: &str =
    Layout::RUNTIME_COMPOSE_DSL_WEBVIEW_FILES_DIR_PATH;
pub const RUNTIME_CLIENT_LOG_PATH: &str = Layout::RUNTIME_CLIENT_LOG_PATH;
pub const RUNTIME_SHARE_IMAGE_EXPORTS_DIR_PATH: &str = Layout::RUNTIME_SHARE_IMAGE_EXPORTS_DIR_PATH;

pub const EXTENSIONS_SKILLS_DIR_PATH: &str = Layout::EXTENSIONS_SKILLS_DIR_PATH;
pub const EXTENSIONS_PACKAGES_DIR_PATH: &str = Layout::EXTENSIONS_PACKAGES_DIR_PATH;
pub const EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH: &str = Layout::EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH;
pub const EXTENSIONS_MCP_DIR_PATH: &str = Layout::EXTENSIONS_MCP_DIR_PATH;

pub const RUNTIME_CLEAN_ON_EXIT_DIR_PATH: &str = Layout::RUNTIME_CLEAN_ON_EXIT_DIR_PATH;
pub const RUNTIME_TOOLPKG_RESOURCE_EXPORTS_DIR_PATH: &str =
    Layout::RUNTIME_TOOLPKG_RESOURCE_EXPORTS_DIR_PATH;
pub const RUNTIME_TOOLPKG_RESOURCE_EXPORTS_INTERNAL_DIR_PATH: &str =
    Layout::RUNTIME_TOOLPKG_RESOURCE_EXPORTS_INTERNAL_DIR_PATH;
pub const RUNTIME_WEBSESSION_USERSCRIPTS_STATE_PATH: &str =
    Layout::RUNTIME_WEBSESSION_USERSCRIPTS_STATE_PATH;
pub const RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH: &str =
    Layout::RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH;
pub const RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH: &str =
    Layout::RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH;
pub const RUNTIME_WEBSESSION_BROWSER_DOWNLOADS_PATH: &str =
    Layout::RUNTIME_WEBSESSION_BROWSER_DOWNLOADS_PATH;
pub const RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES_DIR_PATH: &str =
    Layout::RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES_DIR_PATH;

pub const EXPORTS_DIR_PATH: &str = Layout::EXPORTS_DIR_PATH;
pub const WORKSPACE_DIR_PATH: &str = Layout::WORKSPACE_DIR_PATH;
pub const OPERIT_LOG_PATH: &str = Layout::OPERIT_LOG_PATH;
pub const TOOLPKG_LOG_PATH: &str = Layout::TOOLPKG_LOG_PATH;

pub const USER_PREFERENCES_PATH: &str = Layout::USER_PREFERENCES_PATH;
pub const API_PREFERENCES_PATH: &str = Layout::API_PREFERENCES_PATH;
pub const ENV_PREFERENCES_PATH: &str = Layout::ENV_PREFERENCES_PATH;
pub const GITHUB_AUTH_PREFERENCES_PATH: &str = Layout::GITHUB_AUTH_PREFERENCES_PATH;
pub const CODEX_AUTH_PREFERENCES_PATH: &str = Layout::CODEX_AUTH_PREFERENCES_PATH;
pub const MODEL_CONFIGS_PREFERENCES_PATH: &str = Layout::MODEL_CONFIGS_PREFERENCES_PATH;
pub const FUNCTIONAL_CONFIGS_PREFERENCES_PATH: &str = Layout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH;
pub const PACKAGE_MANAGER_PREFERENCES_PATH: &str = Layout::PACKAGE_MANAGER_PREFERENCES_PATH;
pub const CURRENT_CHAT_ID_PREFERENCES_PATH: &str = Layout::CURRENT_CHAT_ID_PREFERENCES_PATH;
pub const SQLITE_DATABASE_PATH: &str = Layout::SQLITE_DATABASE_PATH;
pub const MCP_CONFIG_PATH: &str = Layout::MCP_CONFIG_PATH;
pub const MCP_SERVER_STATUS_PATH: &str = Layout::MCP_SERVER_STATUS_PATH;


#[allow(non_snake_case)]
pub fn operitRootDir() -> Result<PathBuf, String> {
    Ok(default_runtime_dir())
}

/// Maps a virtual runtime storage path into a supplied physical runtime root.
#[allow(non_snake_case)]
pub fn runtimePathFromRoot(
    runtimeRoot: &std::path::Path,
    storagePath: &str,
) -> Result<PathBuf, String> {
    let runtimePrefix = format!("{}/", Layout::RUNTIME_ROOT_DIR_PATH);
    let relativePath = storagePath.strip_prefix(&runtimePrefix).ok_or_else(|| {
        format!("runtime storage path must start with {runtimePrefix}: {storagePath}")
    })?;
    Ok(runtimeRoot.join(relativePath))
}

#[allow(non_snake_case)]
pub fn preferencesDir() -> Result<PathBuf, String> {
    relativeDir(CONFIG_PREFERENCES_DIR_PATH)
}



#[allow(non_snake_case)]
/// Returns the directory used for imported UI theme assets.
pub fn themeAssetsDir() -> Result<PathBuf, String> {
    relativeDir(RUNTIME_THEME_ASSETS_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn pluginConfigsDir() -> Result<PathBuf, String> {
    relativeDir(EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn pluginConfigDir(pluginId: &str) -> Result<PathBuf, String> {
    Ok(pluginConfigsDir()?.join(pluginConfigDirName(pluginId)?))
}

/// Resolves the stable config directory name without requiring a process-global runtime root.
#[allow(non_snake_case)]
pub fn pluginConfigDirName(pluginId: &str) -> Result<String, String> {
    let trimmed = pluginId.trim();
    if trimmed.is_empty() {
        return Err("plugin id must not be blank".to_string());
    }
    let safeBaseName = sanitizePluginConfigDirName(trimmed);
    if safeBaseName.is_empty() {
        return Err(format!(
            "plugin id cannot be mapped to a config path: {trimmed}"
        ));
    }
    let safeName = if safeBaseName == trimmed {
        safeBaseName
    } else {
        format!("{safeBaseName}-{:x}", javaStringHashCode(trimmed))
    };
    Ok(safeName)
}

#[allow(non_snake_case)]
pub fn cleanOnExitDir() -> Result<PathBuf, String> {
    relativeDir(RUNTIME_CLEAN_ON_EXIT_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn toolPkgResourceExportsDir(internal: bool) -> Result<PathBuf, String> {
    if internal {
        relativeDir(RUNTIME_TOOLPKG_RESOURCE_EXPORTS_INTERNAL_DIR_PATH)
    } else {
        relativeDir(RUNTIME_TOOLPKG_RESOURCE_EXPORTS_DIR_PATH)
    }
}

#[allow(non_snake_case)]
pub fn exportsDir() -> Result<PathBuf, String> {
    relativeDir(EXPORTS_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn workspaceDir() -> Result<PathBuf, String> {
    Ok(default_workspace_dir())
}

#[allow(non_snake_case)]
pub fn workspacePath(chatId: &str) -> Result<PathBuf, String> {
    let id = chatId.trim();
    if id.is_empty() {
        return Err("chat id must not be blank".to_string());
    }
    Ok(workspaceDir()?.join(id))
}

#[allow(non_snake_case)]
pub fn webSessionUserscriptsStatePath() -> Result<PathBuf, String> {
    relativeFile(RUNTIME_WEBSESSION_USERSCRIPTS_STATE_PATH)
}

#[allow(non_snake_case)]
pub fn webSessionBrowserBookmarksPath() -> Result<PathBuf, String> {
    relativeFile(RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH)
}

#[allow(non_snake_case)]
pub fn webSessionBrowserHistoryPath() -> Result<PathBuf, String> {
    relativeFile(RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH)
}

#[allow(non_snake_case)]
pub fn webSessionBrowserDownloadsPath() -> Result<PathBuf, String> {
    relativeFile(RUNTIME_WEBSESSION_BROWSER_DOWNLOADS_PATH)
}

#[allow(non_snake_case)]
pub fn webSessionBrowserDownloadFilesDir() -> Result<PathBuf, String> {
    relativeDir(RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn rawSnapshotExcludedFilesTopLevelDirNames() -> Vec<String> {
    Vec::new()
}

#[allow(non_snake_case)]
pub fn userPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(USER_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn apiPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(API_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn envPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(ENV_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn githubAuthPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(GITHUB_AUTH_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn codexAuthPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(CODEX_AUTH_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn modelConfigsPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(MODEL_CONFIGS_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn functionalConfigsPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(FUNCTIONAL_CONFIGS_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn packageManagerPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(PACKAGE_MANAGER_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn customPreferencePath(fileName: &str) -> Result<PathBuf, String> {
    let fileName = normalizePlainFileName(fileName)?;
    Ok(preferencesDir()?.join(fileName))
}

/// Builds a host-relative storage path for one custom preference file.
#[allow(non_snake_case)]
pub fn customPreferenceStoragePath(fileName: &str) -> Result<String, String> {
    let fileName = normalizePlainFileName(fileName)?;
    Ok(format!("{CONFIG_PREFERENCES_DIR_PATH}/{fileName}"))
}

#[allow(non_snake_case)]
pub fn currentChatIdPreferencesPath() -> Result<PathBuf, String> {
    relativeFile(CURRENT_CHAT_ID_PREFERENCES_PATH)
}

#[allow(non_snake_case)]
pub fn sqliteDatabasePath() -> Result<PathBuf, String> {
    relativeFile(SQLITE_DATABASE_PATH)
}

#[allow(non_snake_case)]
pub fn mcpConfigPath() -> Result<PathBuf, String> {
    relativeFile(MCP_CONFIG_PATH)
}

#[allow(non_snake_case)]
pub fn mcpServerStatusPath() -> Result<PathBuf, String> {
    relativeFile(MCP_SERVER_STATUS_PATH)
}

#[allow(non_snake_case)]
pub fn cleanOnExitPathSdcard() -> Result<String, String> {
    pathString(cleanOnExitDir()?)
}

#[allow(non_snake_case)]
pub fn operitRootPathSdcard() -> Result<String, String> {
    pathString(operitRootDir()?)
}

#[allow(non_snake_case)]
pub fn exportsPathSdcard() -> Result<String, String> {
    pathString(exportsDir()?)
}

#[allow(non_snake_case)]
pub fn workspacePathSdcard(chatId: &str) -> Result<String, String> {
    pathString(workspacePath(chatId)?)
}

#[allow(non_snake_case)]
fn relativeDir(relativePath: &str) -> Result<PathBuf, String> {
    runtimePathFromRoot(&default_runtime_dir(), relativePath)
}

#[allow(non_snake_case)]
fn relativeFile(relativePath: &str) -> Result<PathBuf, String> {
    runtimePathFromRoot(&default_runtime_dir(), relativePath)
}

#[allow(non_snake_case)]
fn pathString(path: PathBuf) -> Result<String, String> {
    Ok(path.to_string_lossy().to_string())
}

#[allow(non_snake_case)]
fn normalizePlainFileName(fileName: &str) -> Result<String, String> {
    let trimmed = fileName.trim();
    if trimmed.is_empty() {
        return Err("file name must not be blank".to_string());
    }
    if trimmed == "." || trimmed == ".." || trimmed.chars().any(|ch| ch == '/' || ch == '\\') {
        return Err("file name must be a plain file name".to_string());
    }
    Ok(trimmed.to_string())
}

#[allow(non_snake_case)]
fn sanitizePluginConfigDirName(pluginId: &str) -> String {
    pluginId
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch <= '\u{1f}'
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches(|ch| ch == '.' || ch == ' ')
        .to_string()
}

#[allow(non_snake_case)]
fn javaStringHashCode(value: &str) -> i32 {
    value.encode_utf16().fold(0_i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(unit as i32)
    })
}
