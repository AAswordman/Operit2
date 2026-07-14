#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use operit_util::RuntimeStorageLayout::*;
use operit_util::RuntimeStoreRoot::{default_runtime_dir, default_workspace_dir};

#[derive(Clone, Debug)]
/// Resolves persisted runtime and workspace paths from two explicit roots.
pub struct RuntimeStorePaths {
    runtime_dir: PathBuf,
    workspace_dir: PathBuf,
}

impl RuntimeStorePaths {
    /// Creates path helpers from explicit runtime and workspace roots.
    pub fn new(runtime_dir: PathBuf, workspace_dir: PathBuf) -> Self {
        Self {
            runtime_dir,
            workspace_dir,
        }
    }

    /// Creates path helpers from the configured default roots.
    pub fn default() -> Self {
        Self::new(default_runtime_dir(), default_workspace_dir())
    }

    /// Returns the directory used for runtime files.
    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    /// Returns the model configuration preferences path.
    pub fn model_configs_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(MODEL_CONFIGS_PREFERENCES_PATH)
    }

    /// Returns the functional configuration preferences path.
    pub fn functional_configs_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(FUNCTIONAL_CONFIGS_PREFERENCES_PATH)
    }

    /// Returns the installed skills directory.
    pub fn skills_dir(&self) -> PathBuf {
        self.runtime_storage_path(EXTENSIONS_SKILLS_DIR_PATH)
    }

    /// Returns the installed package directory.
    pub fn packages_dir(&self) -> PathBuf {
        self.runtime_storage_path(EXTENSIONS_PACKAGES_DIR_PATH)
    }

    /// Returns the MCP plugin directory.
    pub fn mcp_plugins_dir(&self) -> PathBuf {
        self.runtime_storage_path(EXTENSIONS_MCP_DIR_PATH)
    }

    /// Returns the MCP configuration file path.
    pub fn mcp_config_path(&self) -> PathBuf {
        self.runtime_storage_path(MCP_CONFIG_PATH)
    }

    /// Returns the MCP server status file path.
    pub fn mcp_server_status_path(&self) -> PathBuf {
        self.runtime_storage_path(MCP_SERVER_STATUS_PATH)
    }

    /// Returns the character card preferences path.
    pub fn character_cards_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(CHARACTER_CARDS_PREFERENCES_PATH)
    }

    /// Returns the character group preferences path.
    pub fn character_groups_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(CHARACTER_GROUPS_PREFERENCES_PATH)
    }

    /// Returns the prompt tag preferences path.
    pub fn prompt_tags_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(PROMPT_TAGS_PREFERENCES_PATH)
    }

    /// Returns the shared memory store preferences path.
    pub fn shared_memory_stores_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(SHARED_MEMORY_STORES_PREFERENCES_PATH)
    }

    /// Returns the TTS configuration preferences path.
    pub fn tts_configs_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(TTS_CONFIGS_PREFERENCES_PATH)
    }

    /// Returns the STT configuration preferences path.
    pub fn stt_configs_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(STT_CONFIGS_PREFERENCES_PATH)
    }

    /// Returns the tool permission preferences path.
    pub fn tool_permissions_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(TOOL_PERMISSIONS_PREFERENCES_PATH)
    }

    /// Returns the skill visibility preferences path.
    pub fn skill_visibility_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(SKILL_VISIBILITY_PREFERENCES_PATH)
    }

    /// Returns the package manager preferences path.
    pub fn package_manager_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(PACKAGE_MANAGER_PREFERENCES_PATH)
    }

    /// Returns the current chat id state preferences path.
    pub fn current_chat_id_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(CURRENT_CHAT_ID_PREFERENCES_PATH)
    }

    /// Returns the main SQLite database path.
    pub fn sqlite_database_path(&self) -> PathBuf {
        self.runtime_storage_path(SQLITE_DATABASE_PATH)
    }

    /// Returns the root workspace directory.
    pub fn workspace_dir(&self) -> PathBuf {
        self.workspace_dir.clone()
    }

    /// Returns the workspace directory bound to a chat identifier.
    pub fn workspace_path(&self, chat_id: &str) -> PathBuf {
        self.workspace_dir().join(chat_id)
    }

    /// Returns the runtime sync operation directory.
    pub fn sync_dir(&self) -> PathBuf {
        self.runtime_storage_path(RUNTIME_SYNC_DIR_PATH)
    }

    /// Returns the adjacent sync directory used by legacy sync files.
    pub fn adjacent_sync_dir(&self) -> PathBuf {
        self.runtime_dir.join("sync")
    }

    /// Returns the model connection test cache directory.
    pub fn model_connection_test_cache_dir(&self) -> PathBuf {
        self.runtime_storage_path(RUNTIME_MODEL_CONNECTION_TEST_CACHE_DIR_PATH)
    }

    /// Returns the tool package cache directory.
    pub fn toolpkg_cache_dir(&self) -> PathBuf {
        self.runtime_storage_path(RUNTIME_TOOLPKG_CACHE_DIR_PATH)
    }

    /// Returns the synthesized TTS audio cache directory.
    pub fn tts_audio_dir(&self) -> PathBuf {
        self.runtime_storage_path(RUNTIME_TTS_AUDIO_DIR_PATH)
    }

    /// Ensures the package extension directory exists.
    pub fn ensure_packages_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.packages_dir())
    }

    /// Ensures the MCP extension directory exists.
    pub fn ensure_mcp_plugins_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.mcp_plugins_dir())
    }

    /// Maps a virtual runtime storage path into the configured physical runtime root.
    pub fn runtime_storage_path(&self, storage_path: &str) -> PathBuf {
        Self::runtime_storage_path_from_root(&self.runtime_dir, storage_path)
    }

    /// Maps a virtual runtime storage path into an explicit physical runtime root.
    pub fn runtime_storage_path_from_root(runtime_dir: &Path, storage_path: &str) -> PathBuf {
        let runtime_prefix = format!("{RUNTIME_ROOT_DIR_PATH}/");
        let relative = storage_path
            .strip_prefix(&runtime_prefix)
            .expect("runtime storage path must start with runtime/");
        runtime_dir.join(relative)
    }
}

#[allow(non_snake_case)]
#[cfg(not(target_arch = "wasm32"))]
fn ensureRuntimeDirectory(path: PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(path)
}

#[allow(non_snake_case)]
#[cfg(target_arch = "wasm32")]
fn ensureRuntimeDirectory(_path: PathBuf) -> std::io::Result<()> {
    Ok(())
}
