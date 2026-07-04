#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::RuntimeStorageLayout::*;

#[derive(Clone, Debug)]
pub struct RuntimeStorePaths {
    root_dir: PathBuf,
}

impl RuntimeStorePaths {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn default() -> Self {
        Self::new(default_data_dir())
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn model_configs_preferences_path(&self) -> PathBuf {
        self.root_dir.join(MODEL_CONFIGS_PREFERENCES_PATH)
    }

    pub fn functional_configs_preferences_path(&self) -> PathBuf {
        self.root_dir.join(FUNCTIONAL_CONFIGS_PREFERENCES_PATH)
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root_dir.join(EXTENSIONS_SKILLS_DIR_PATH)
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.root_dir.join(EXTENSIONS_PACKAGES_DIR_PATH)
    }

    pub fn mcp_plugins_dir(&self) -> PathBuf {
        self.root_dir.join(EXTENSIONS_MCP_DIR_PATH)
    }

    pub fn mcp_config_path(&self) -> PathBuf {
        self.root_dir.join(MCP_CONFIG_PATH)
    }

    pub fn mcp_server_status_path(&self) -> PathBuf {
        self.root_dir.join(MCP_SERVER_STATUS_PATH)
    }

    pub fn character_cards_preferences_path(&self) -> PathBuf {
        self.root_dir.join(CHARACTER_CARDS_PREFERENCES_PATH)
    }

    pub fn character_groups_preferences_path(&self) -> PathBuf {
        self.root_dir.join(CHARACTER_GROUPS_PREFERENCES_PATH)
    }

    pub fn prompt_tags_preferences_path(&self) -> PathBuf {
        self.root_dir.join(PROMPT_TAGS_PREFERENCES_PATH)
    }

    pub fn shared_memory_stores_preferences_path(&self) -> PathBuf {
        self.root_dir.join(SHARED_MEMORY_STORES_PREFERENCES_PATH)
    }

    pub fn tts_configs_preferences_path(&self) -> PathBuf {
        self.root_dir.join(TTS_CONFIGS_PREFERENCES_PATH)
    }

    pub fn tool_permissions_preferences_path(&self) -> PathBuf {
        self.root_dir.join(TOOL_PERMISSIONS_PREFERENCES_PATH)
    }

    pub fn skill_visibility_preferences_path(&self) -> PathBuf {
        self.root_dir.join(SKILL_VISIBILITY_PREFERENCES_PATH)
    }

    pub fn package_manager_preferences_path(&self) -> PathBuf {
        self.root_dir.join(PACKAGE_MANAGER_PREFERENCES_PATH)
    }

    pub fn current_chat_id_preferences_path(&self) -> PathBuf {
        self.root_dir.join(CURRENT_CHAT_ID_PREFERENCES_PATH)
    }

    pub fn sqlite_database_path(&self) -> PathBuf {
        self.root_dir.join(SQLITE_DATABASE_PATH)
    }

    pub fn workspace_dir(&self) -> PathBuf {
        self.root_dir.join(WORKSPACE_DIR_PATH)
    }

    pub fn workspace_path(&self, chat_id: &str) -> PathBuf {
        self.workspace_dir().join(chat_id)
    }

    pub fn sync_dir(&self) -> PathBuf {
        self.root_dir.join(RUNTIME_SYNC_DIR_PATH)
    }

    pub fn adjacent_sync_dir(&self) -> PathBuf {
        self.root_dir.join("sync")
    }

    pub fn model_connection_test_cache_dir(&self) -> PathBuf {
        self.root_dir
            .join(RUNTIME_MODEL_CONNECTION_TEST_CACHE_DIR_PATH)
    }

    pub fn toolpkg_cache_dir(&self) -> PathBuf {
        self.root_dir.join(RUNTIME_TOOLPKG_CACHE_DIR_PATH)
    }

    pub fn tts_audio_dir(&self) -> PathBuf {
        self.root_dir.join(RUNTIME_TTS_AUDIO_DIR_PATH)
    }

    pub fn ensure_packages_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.packages_dir())
    }

    pub fn ensure_mcp_plugins_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.mcp_plugins_dir())
    }
}

#[allow(non_snake_case)]
pub fn setDefaultRuntimeStoreRoot(root_dir: PathBuf) {
    let holder = DEFAULT_RUNTIME_STORE_ROOT.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default runtime store root mutex poisoned");
    *guard = Some(root_dir);
}

fn default_runtime_store_root() -> Option<PathBuf> {
    let holder = DEFAULT_RUNTIME_STORE_ROOT.get_or_init(|| Mutex::new(None));
    holder
        .lock()
        .expect("default runtime store root mutex poisoned")
        .clone()
}

pub fn default_data_dir() -> PathBuf {
    default_runtime_store_root().expect("default runtime store root is not registered")
}

static DEFAULT_RUNTIME_STORE_ROOT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

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
