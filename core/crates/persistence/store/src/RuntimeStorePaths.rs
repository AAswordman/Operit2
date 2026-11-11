use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use operit_util::RuntimeStorageLayout::*;
use operit_util::RuntimeStoreRoot::{default_runtime_dir, default_workspace_dir};

#[derive(Clone, Debug)]
/// Resolves persisted runtime and workspace paths from two explicit roots.
pub struct RuntimeStorePaths;

impl RuntimeStorePaths {
    /// Creates path helpers for the canonical virtual storage namespaces.
    pub fn default() -> Self { Self }
    /// Returns the directory used for runtime files.
    pub fn runtime_dir(&self) -> &Path {
        Path::new("/operitapp/data")
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



    /// Returns the named workspace preferences path.
    pub fn workspaces_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(WORKSPACES_PREFERENCES_PATH)
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

    /// Returns the CoreNode-local ToolPkg installation preferences path.
    pub fn toolpkg_installation_preferences_path(&self) -> PathBuf {
        self.runtime_storage_path(TOOLPKG_INSTALLATION_PREFERENCES_PATH)
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
        default_workspace_dir()
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
        self.runtime_dir().join("sync")
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

    /// Returns the canonical VFS address of a runtime storage key.
    pub fn runtime_storage_path(&self, storage_path: &str) -> PathBuf {
        operit_util::OperitPaths::runtimePath(storage_path).expect("runtime storage key must be valid")
    }
    /// Converts one physical runtime path into its host-relative storage key.
    pub fn runtime_storage_key(&self, path: &Path) -> Result<String, String> {
        let relative = path.strip_prefix(Path::new("/operitapp/data")).map_err(|_| {
            format!(
                "runtime path {} is outside {}",
                path.display(),
                self.runtime_dir().display()
            )
        })?;
        let mut segments = Vec::new();
        for component in relative.components() {
            match component {
                std::path::Component::Normal(segment) => {
                    segments.push(segment.to_string_lossy().into_owned())
                }
                _ => {
                    return Err(format!(
                        "runtime path contains an invalid component: {}",
                        path.display()
                    ))
                }
            }
        }
        if segments.is_empty() {
            return Err("runtime storage key must identify an entry below runtime".to_string());
        }
        Ok(format!("{RUNTIME_ROOT_DIR_PATH}/{}", segments.join("/")))
    }

}
