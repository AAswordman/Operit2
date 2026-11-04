//! Stable, inheritable output bindings; folder names are allocated once, never per day.
#![allow(non_snake_case)]
use crate::{
    FileSyncPolicy::{readLimited, FileSyncPolicy},
    RuntimeFileSyncStore::RuntimeFileSyncStore,
};
use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{
    RUNTIME_CHAT_OUTPUTS_DIR_PATH, RUNTIME_CHAT_OUTPUT_BINDINGS_DIR_PATH, RUNTIME_SYNC_DIR_PATH,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    version: u32,
    directory: String,
}
#[derive(Clone)]
pub struct ChatOutputStore {
    host: Arc<dyn RuntimeStorageHost>,
}
impl ChatOutputStore {
    pub fn new(host: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { host }
    }
    fn bindingPath(chatId: &str) -> Result<String, String> {
        if chatId.is_empty() || chatId.contains(['/', '\\', '\0']) || matches!(chatId, "." | "..") {
            return Err("Invalid chat output binding id".into());
        }
        Ok(format!(
            "{RUNTIME_CHAT_OUTPUT_BINDINGS_DIR_PATH}/{chatId}.json"
        ))
    }
    pub fn directory(&self, chatId: &str) -> Result<Option<String>, String> {
        let path = Self::bindingPath(chatId)?;
        if !self.host.exists(&path).map_err(|e| e.to_string())? {
            return Ok(None);
        }
        let binding: Binding =
            serde_json::from_slice(&readLimited(self.host.as_ref(), &path, 4096)?)
                .map_err(|e| format!("Invalid output binding: {e}"))?;
        if binding.version != 1 {
            return Err("Unsupported output binding version".into());
        }
        Self::storagePath(&binding.directory)?;
        Ok(Some(binding.directory))
    }
    pub fn bind(
        &self,
        chatId: &str,
        createdAt: i64,
        inherited: Option<&str>,
    ) -> Result<String, String> {
        if let Some(directory) = self.directory(chatId)? {
            return Ok(directory);
        }
        let directory = match inherited {
            Some(path) => {
                Self::storagePath(path)?;
                path.to_string()
            }
            None => {
                let date = chrono::DateTime::from_timestamp_millis(createdAt)
                    .ok_or("Invalid conversation creation time")?
                    .with_timezone(&chrono::Local);
                format!(
                    "/operitapp/outputs/{}",
                    date.format("%Y-%m-%d_%H-%M-%S-%3f")
                )
            }
        };
        let bytes = serde_json::to_vec(&Binding {
            version: 1,
            directory: directory.clone(),
        })
        .map_err(|e| e.to_string())?;
        RuntimeFileSyncStore::new(self.host.clone(), RUNTIME_SYNC_DIR_PATH)
            .writeBytes(&Self::bindingPath(chatId)?, &bytes)?;
        Ok(directory)
    }
    pub fn prepare(&self, chatId: &str) -> Result<Option<String>, String> {
        let directory = self.directory(chatId)?;
        if let Some(directory) = &directory {
            FileSyncPolicy::ensure(self.host.as_ref(), &Self::storagePath(directory)?)?;
        }
        Ok(directory)
    }
    pub fn storagePath(directory: &str) -> Result<String, String> {
        let name = directory
            .strip_prefix("/operitapp/outputs/")
            .ok_or("Invalid conversation output directory")?;
        if name.is_empty() || name.contains(['/', '\\', '\0']) || matches!(name, "." | "..") {
            return Err("Invalid conversation output directory name".into());
        }
        Ok(format!("{RUNTIME_CHAT_OUTPUTS_DIR_PATH}/{name}"))
    }
}
