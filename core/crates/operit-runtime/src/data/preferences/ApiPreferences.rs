use std::path::PathBuf;

use operit_store::RuntimeStorePaths::default_data_dir;

pub struct ApiPreferences;

impl ApiPreferences {
    pub const DEFAULT_API_KEY: &'static str = "";
    pub const DEFAULT_API_ENDPOINT: &'static str = "https://api.deepseek.com/v1/chat/completions";
    pub const DEFAULT_MODEL_NAME: &'static str = "deepseek-chat";
    pub const DEFAULT_CONFIG_ID: &'static str = "default";
    pub const DEFAULT_CONFIG_NAME: &'static str = "model_config_default_name";

    pub fn data_dir() -> PathBuf {
        default_data_dir()
    }
}
