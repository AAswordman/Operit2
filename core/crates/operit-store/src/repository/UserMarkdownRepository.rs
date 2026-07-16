use std::path::PathBuf;
use std::sync::Arc;

use crate::RuntimeStorageHost::runtimeStoragePath;
use operit_host_api::RuntimeStorageHost;
use operit_util::OperitPaths::userMarkdownPath;

#[derive(Clone)]
pub struct UserMarkdownRepository {
    ownerKey: String,
    storageHost: Arc<dyn RuntimeStorageHost>,
}

impl std::fmt::Debug for UserMarkdownRepository {
    /// Formats repository identity without exposing its host implementation.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UserMarkdownRepository")
            .field("ownerKey", &self.ownerKey)
            .finish()
    }
}

impl UserMarkdownRepository {
    pub const FILE_NAME: &'static str = "USER.md";

    /// Creates a user markdown repository for the given owner key.
    pub fn new(ownerKey: impl Into<String>, storageHost: Arc<dyn RuntimeStorageHost>) -> Self {
        Self {
            ownerKey: ownerKey.into(),
            storageHost,
        }
    }

    #[allow(non_snake_case)]
    /// Returns the owner key used to locate the user markdown file.
    pub fn ownerKey(&self) -> &str {
        &self.ownerKey
    }

    #[allow(non_snake_case)]
    /// Returns the filesystem path of the user markdown file.
    pub fn userMarkdownPath(&self) -> Result<PathBuf, String> {
        userMarkdownPath(&self.ownerKey)
    }

    #[allow(non_snake_case)]
    /// Reads the user markdown file after ensuring it exists.
    pub fn readUserMarkdown(&self) -> Result<String, String> {
        self.ensureUserMarkdown()?;
        let content = self
            .storageHost
            .readBytes(&self.storagePath()?)
            .map_err(|error| error.to_string())?;
        String::from_utf8(content).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Writes normalized content to the user markdown file.
    pub fn writeUserMarkdown(&self, content: String) -> Result<(), String> {
        self.storageHost
            .writeBytes(&self.storagePath()?, normalizeMarkdown(content).as_bytes())
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn ensureUserMarkdown(&self) -> Result<(), String> {
        let path = self.storagePath()?;
        if !self
            .storageHost
            .exists(&path)
            .map_err(|error| error.to_string())?
        {
            self.storageHost
                .writeBytes(&path, b"# USER\n\n")
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Maps the user markdown path into its host-owned runtime storage key.
    fn storagePath(&self) -> Result<String, String> {
        Ok(runtimeStoragePath(&userMarkdownPath(&self.ownerKey)?))
    }
}

fn normalizeMarkdown(content: String) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        "# USER\n\n".to_string()
    } else {
        format!("{trimmed}\n")
    }
}
