use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
        fs::create_dir_all(&self.root_dir)
    }

    pub fn ensure_chats_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.chats_dir())
    }

    pub fn ensure_skills_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.skills_dir())
    }

    pub fn ensure_packages_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.packages_dir())
    }

    pub fn ensure_mcp_plugins_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.mcp_plugins_dir())
    }
}

pub fn default_data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let appdata = env::var_os("APPDATA").expect("APPDATA is required for Operit2 runtime storage");
        return PathBuf::from(appdata).join("Operit2");
    }
    if cfg!(target_os = "macos") {
        let home = env::var_os("HOME").expect("HOME is required for Operit2 runtime storage");
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Operit2");
    }
    if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg_data_home).join("operit2");
    }
    let home = env::var_os("HOME").expect("HOME is required for Operit2 runtime storage");
    PathBuf::from(home).join(".local").join("share").join("operit2")
}
