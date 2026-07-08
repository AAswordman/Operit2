#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use operit_util::RuntimeStorageLayout::*;
use operit_util::RuntimeStoreRoot::default_data_dir;

#[derive(Clone, Debug)]
/// Resolves all persisted runtime data paths from one registered root directory.
pub struct RuntimeStorePaths {
    root_dir: PathBuf,
}

impl RuntimeStorePaths {
    /// Creates path helpers rooted at the supplied data directory.
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    /// Creates path helpers rooted at the configured default data directory.
    pub fn default() -> Self {
        Self::new(default_data_dir())
    }

    /// Returns the root directory used to resolve all runtime store paths.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Returns the model configuration preferences path.
    pub fn model_configs_preferences_path(&self) -> PathBuf {
        self.root_dir.join(MODEL_CONFIGS_PREFERENCES_PATH)
    }

    /// Returns the functional configuration preferences path.
    pub fn functional_configs_preferences_path(&self) -> PathBuf {
        self.root_dir.join(FUNCTIONAL_CONFIGS_PREFERENCES_PATH)
    }

    /// Returns the installed skills directory.
    pub fn skills_dir(&self) -> PathBuf {
        self.root_dir.join(EXTENSIONS_SKILLS_DIR_PATH)
    }

    /// Returns the installed package directory.
    pub fn packages_dir(&self) -> PathBuf {
        self.root_dir.join(EXTENSIONS_PACKAGES_DIR_PATH)
    }

    /// Returns the MCP plugin directory.
    pub fn mcp_plugins_dir(&self) -> PathBuf {
        self.root_dir.join(EXTENSIONS_MCP_DIR_PATH)
    }

    /// Returns the MCP configuration file path.
    pub fn mcp_config_path(&self) -> PathBuf {
        self.root_dir.join(MCP_CONFIG_PATH)
    }

    /// Returns the MCP server status file path.
    pub fn mcp_server_status_path(&self) -> PathBuf {
        self.root_dir.join(MCP_SERVER_STATUS_PATH)
    }

    /// Returns the character card preferences path.
    pub fn character_cards_preferences_path(&self) -> PathBuf {
        self.root_dir.join(CHARACTER_CARDS_PREFERENCES_PATH)
    }

    /// Returns the character group preferences path.
    pub fn character_groups_preferences_path(&self) -> PathBuf {
        self.root_dir.join(CHARACTER_GROUPS_PREFERENCES_PATH)
    }

    /// Returns the prompt tag preferences path.
    pub fn prompt_tags_preferences_path(&self) -> PathBuf {
        self.root_dir.join(PROMPT_TAGS_PREFERENCES_PATH)
    }

    /// Returns the shared memory store preferences path.
    pub fn shared_memory_stores_preferences_path(&self) -> PathBuf {
        self.root_dir.join(SHARED_MEMORY_STORES_PREFERENCES_PATH)
    }

    /// Returns the TTS configuration preferences path.
    pub fn tts_configs_preferences_path(&self) -> PathBuf {
        self.root_dir.join(TTS_CONFIGS_PREFERENCES_PATH)
    }

    /// Returns the tool permission preferences path.
    pub fn tool_permissions_preferences_path(&self) -> PathBuf {
        self.root_dir.join(TOOL_PERMISSIONS_PREFERENCES_PATH)
    }

    /// Returns the skill visibility preferences path.
    pub fn skill_visibility_preferences_path(&self) -> PathBuf {
        self.root_dir.join(SKILL_VISIBILITY_PREFERENCES_PATH)
    }

    /// Returns the package manager preferences path.
    pub fn package_manager_preferences_path(&self) -> PathBuf {
        self.root_dir.join(PACKAGE_MANAGER_PREFERENCES_PATH)
    }

    /// Returns the current chat id state preferences path.
    pub fn current_chat_id_preferences_path(&self) -> PathBuf {
        self.root_dir.join(CURRENT_CHAT_ID_PREFERENCES_PATH)
    }

    /// Returns the main SQLite database path.
    pub fn sqlite_database_path(&self) -> PathBuf {
        self.root_dir.join(SQLITE_DATABASE_PATH)
    }

    /// Returns the root workspace directory.
    pub fn workspace_dir(&self) -> PathBuf {
        self.root_dir.join(WORKSPACE_DIR_PATH)
    }

    /// Returns the workspace directory bound to a chat identifier.
    pub fn workspace_path(&self, chat_id: &str) -> PathBuf {
        self.workspace_dir().join(chat_id)
    }

    /// Returns the runtime sync operation directory.
    pub fn sync_dir(&self) -> PathBuf {
        self.root_dir.join(RUNTIME_SYNC_DIR_PATH)
    }

    /// Returns the adjacent sync directory used by legacy sync files.
    pub fn adjacent_sync_dir(&self) -> PathBuf {
        self.root_dir.join("sync")
    }

    /// Returns the model connection test cache directory.
    pub fn model_connection_test_cache_dir(&self) -> PathBuf {
        self.root_dir
            .join(RUNTIME_MODEL_CONNECTION_TEST_CACHE_DIR_PATH)
    }

    /// Returns the tool package cache directory.
    pub fn toolpkg_cache_dir(&self) -> PathBuf {
        self.root_dir.join(RUNTIME_TOOLPKG_CACHE_DIR_PATH)
    }

    /// Returns the synthesized TTS audio cache directory.
    pub fn tts_audio_dir(&self) -> PathBuf {
        self.root_dir.join(RUNTIME_TTS_AUDIO_DIR_PATH)
    }

    /// Ensures the package extension directory exists.
    pub fn ensure_packages_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.packages_dir())
    }

    /// Ensures the MCP extension directory exists.
    pub fn ensure_mcp_plugins_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.mcp_plugins_dir())
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
