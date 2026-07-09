use std::fs;
use std::path::PathBuf;

use operit_util::OperitPaths::{memoryStoreRootPath, userMarkdownPath};

#[derive(Clone, Debug)]
pub struct UserMarkdownRepository {
    ownerKey: String,
}

impl UserMarkdownRepository {
    pub const FILE_NAME: &'static str = "USER.md";

    /// Creates a user markdown repository for the given owner key.
    pub fn new(ownerKey: impl Into<String>) -> Self {
        Self {
            ownerKey: ownerKey.into(),
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
        fs::read_to_string(self.userMarkdownPath()?).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Writes normalized content to the user markdown file.
    pub fn writeUserMarkdown(&self, content: String) -> Result<(), String> {
        let root = memoryStoreRootPath(&self.ownerKey)?;
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        fs::write(
            userMarkdownPath(&self.ownerKey)?,
            normalizeMarkdown(content),
        )
        .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn ensureUserMarkdown(&self) -> Result<(), String> {
        let root = memoryStoreRootPath(&self.ownerKey)?;
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        let path = userMarkdownPath(&self.ownerKey)?;
        if !path.exists() {
            fs::write(path, "# USER\n\n").map_err(|error| error.to_string())?;
        }
        Ok(())
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
