use std::path::PathBuf;

use operit_store::RuntimeStorageLayout as Layout;
use operit_store::RuntimeStorePaths::default_data_dir;

#[derive(Debug, Clone, Default)]
pub struct OperitPaths;

pub const CONFIG_PREFERENCES_DIR_PATH: &str = Layout::CONFIG_PREFERENCES_DIR_PATH;

pub const DATA_MEMORY_CHARACTERS_DIR_PATH: &str = Layout::DATA_MEMORY_CHARACTERS_DIR_PATH;
pub const DATA_MEMORY_SHARED_DIR_PATH: &str = Layout::DATA_MEMORY_SHARED_DIR_PATH;

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
pub const MODEL_CONFIGS_PREFERENCES_PATH: &str = Layout::MODEL_CONFIGS_PREFERENCES_PATH;
pub const FUNCTIONAL_CONFIGS_PREFERENCES_PATH: &str = Layout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH;
pub const PACKAGE_MANAGER_PREFERENCES_PATH: &str = Layout::PACKAGE_MANAGER_PREFERENCES_PATH;
pub const CURRENT_CHAT_ID_PREFERENCES_PATH: &str = Layout::CURRENT_CHAT_ID_PREFERENCES_PATH;
pub const SQLITE_DATABASE_PATH: &str = Layout::SQLITE_DATABASE_PATH;
pub const MCP_CONFIG_PATH: &str = Layout::MCP_CONFIG_PATH;
pub const MCP_SERVER_STATUS_PATH: &str = Layout::MCP_SERVER_STATUS_PATH;

const MEMORY_USER_MARKDOWN_FILE_NAME: &str = "USER.md";
const MEMORY_SQLITE_FILE_NAME: &str = "Memory.sqlite";
const MEMORY_LINK_SQLITE_FILE_NAME: &str = "MemoryLink.sqlite";
const MEMORY_SEARCH_SETTINGS_FILE_PATH: &str = "settings/memory_search_settings.preferences.json";

#[allow(non_snake_case)]
pub fn operitRootDir() -> Result<PathBuf, String> {
    ensureDir(default_data_dir())
}

#[allow(non_snake_case)]
pub fn preferencesDir() -> Result<PathBuf, String> {
    relativeDir(CONFIG_PREFERENCES_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn memoryCharactersDir() -> Result<PathBuf, String> {
    relativeDir(DATA_MEMORY_CHARACTERS_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn memorySharedDir() -> Result<PathBuf, String> {
    relativeDir(DATA_MEMORY_SHARED_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn pluginConfigsDir() -> Result<PathBuf, String> {
    relativeDir(EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH)
}

#[allow(non_snake_case)]
pub fn pluginConfigDir(pluginId: &str) -> Result<PathBuf, String> {
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
    ensureDir(pluginConfigsDir()?.join(safeName))
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
    relativeDir(WORKSPACE_DIR_PATH)
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
pub fn memoryStoreRootPath(ownerKey: &str) -> Result<PathBuf, String> {
    let owner = parseMemoryOwnerKey(ownerKey)?;
    let ownerRelativePath = match owner.kind {
        MemoryOwnerKind::Character => format!(
            "{}/{}",
            DATA_MEMORY_CHARACTERS_DIR_PATH,
            sanitizeMemoryOwnerId(&owner.id)
        ),
        MemoryOwnerKind::Shared => format!(
            "{}/{}",
            DATA_MEMORY_SHARED_DIR_PATH,
            sanitizeMemoryOwnerId(&owner.id)
        ),
    };
    relativeDir(&ownerRelativePath)
}

#[allow(non_snake_case)]
pub fn userMarkdownPath(ownerKey: &str) -> Result<PathBuf, String> {
    Ok(memoryStoreRootPath(ownerKey)?.join(MEMORY_USER_MARKDOWN_FILE_NAME))
}

#[allow(non_snake_case)]
pub fn memorySqlitePath(ownerKey: &str) -> Result<PathBuf, String> {
    Ok(memoryStoreRootPath(ownerKey)?.join(MEMORY_SQLITE_FILE_NAME))
}

#[allow(non_snake_case)]
pub fn memoryLinkSqlitePath(ownerKey: &str) -> Result<PathBuf, String> {
    Ok(memoryStoreRootPath(ownerKey)?.join(MEMORY_LINK_SQLITE_FILE_NAME))
}

#[allow(non_snake_case)]
pub fn memorySearchSettingsPath(ownerKey: &str) -> Result<PathBuf, String> {
    let path = memoryStoreRootPath(ownerKey)?.join(MEMORY_SEARCH_SETTINGS_FILE_PATH);
    ensureParentDir(&path)?;
    Ok(path)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryOwnerKind {
    Character,
    Shared,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedMemoryOwnerKey {
    pub kind: MemoryOwnerKind,
    pub id: String,
}

#[allow(non_snake_case)]
pub fn parseMemoryOwnerKey(ownerKey: &str) -> Result<ParsedMemoryOwnerKey, String> {
    let (kind, id) = ownerKey
        .split_once(':')
        .ok_or_else(|| format!("invalid memory owner key: {ownerKey}"))?;
    let trimmedId = id.trim();
    if trimmedId.is_empty() {
        return Err(format!("invalid memory owner id: {ownerKey}"));
    }
    let kind = match kind {
        "character" => MemoryOwnerKind::Character,
        "shared" => MemoryOwnerKind::Shared,
        other => return Err(format!("invalid memory owner kind: {other}")),
    };
    Ok(ParsedMemoryOwnerKey {
        kind,
        id: trimmedId.to_string(),
    })
}

#[allow(non_snake_case)]
pub fn characterMemoryOwnerKey(characterCardId: &str) -> Result<String, String> {
    let id = characterCardId.trim();
    if id.is_empty() {
        return Err("character memory owner id is empty".to_string());
    }
    Ok(format!("character:{id}"))
}

#[allow(non_snake_case)]
pub fn sharedMemoryOwnerKey(sharedMemoryId: &str) -> Result<String, String> {
    let id = sharedMemoryId.trim();
    if id.is_empty() {
        return Err("shared memory owner id is empty".to_string());
    }
    Ok(format!("shared:{id}"))
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
pub fn cleanOnExitCleanup() -> Result<(), String> {
    let dir = cleanOnExitDir()?;
    for entry in std::fs::read_dir(&dir)
        .map_err(|error| format!("cannot read cleanOnExit dir {}: {error}", dir.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot read cleanOnExit entry: {error}"))?
            .path();
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
            format!(
                "cannot inspect cleanOnExit entry {}: {error}",
                path.display()
            )
        })?;
        if metadata.is_dir() {
            std::fs::remove_dir_all(&path).map_err(|error| {
                format!("cannot remove cleanOnExit dir {}: {error}", path.display())
            })?;
        } else {
            std::fs::remove_file(&path).map_err(|error| {
                format!("cannot remove cleanOnExit file {}: {error}", path.display())
            })?;
        }
    }
    Ok(())
}

#[allow(non_snake_case)]
fn relativeDir(relativePath: &str) -> Result<PathBuf, String> {
    ensureDir(operitRootDir()?.join(relativePath))
}

#[allow(non_snake_case)]
fn relativeFile(relativePath: &str) -> Result<PathBuf, String> {
    let path = operitRootDir()?.join(relativePath);
    ensureParentDir(&path)?;
    Ok(path)
}

#[allow(non_snake_case)]
fn ensureParentDir(path: &PathBuf) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create dir {}: {error}", parent.display()))
}

#[allow(non_snake_case)]
fn ensureDir(dir: PathBuf) -> Result<PathBuf, String> {
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("cannot create dir {}: {error}", dir.display()))?;
    Ok(dir)
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

#[allow(non_snake_case)]
pub fn sanitizeMemoryOwnerId(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "_".to_string()
    } else {
        out
    }
}
