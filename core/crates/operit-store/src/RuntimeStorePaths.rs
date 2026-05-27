#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

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
        self.root_dir.join("model_configs.preferences.json")
    }

    pub fn functional_configs_preferences_path(&self) -> PathBuf {
        self.root_dir.join("functional_configs.preferences.json")
    }

    pub fn chats_dir(&self) -> PathBuf {
        self.root_dir.join("chats")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root_dir.join("skills")
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.root_dir.join("packages")
    }

    pub fn mcp_plugins_dir(&self) -> PathBuf {
        self.root_dir.join("mcp_plugins")
    }

    pub fn mcp_config_path(&self) -> PathBuf {
        self.mcp_plugins_dir().join("mcp_config.json")
    }

    pub fn mcp_server_status_path(&self) -> PathBuf {
        self.mcp_plugins_dir().join("server_status.json")
    }

    pub fn package_manager_preferences_path(&self) -> PathBuf {
        self.root_dir
            .join("com.ai.assistance.operit.core.tools.PackageManager.preferences.json")
    }

    pub fn chat_path(&self, chat_id: &str) -> PathBuf {
        self.chats_dir().join(format!("{chat_id}.json"))
    }

    pub fn current_chat_id_preferences_path(&self) -> PathBuf {
        self.root_dir.join("current_chat_id.preferences.json")
    }

    pub fn sqlite_database_path(&self) -> PathBuf {
        self.root_dir.join("operit2.sqlite")
    }

    pub fn ensure_root(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.root_dir.clone())
    }

    pub fn ensure_chats_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.chats_dir())
    }

    pub fn ensure_skills_dir(&self) -> std::io::Result<()> {
        ensureRuntimeDirectory(self.skills_dir())
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
