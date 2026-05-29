use std::path::PathBuf;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::FileSystemHost;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::data::dao::ChatDao::ChatDao;
use crate::data::db::AppDatabase::AppDatabase;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileEntry {
    pub name: String,
    pub path: String,
    pub relativePath: String,
    pub isDirectory: bool,
    pub size: i64,
    pub lastModified: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileBytes {
    pub base64Content: String,
}

pub struct WorkspaceService {
    chatDao: ChatDao,
    fileSystemHost: Arc<dyn FileSystemHost>,
}

impl WorkspaceService {
    #[allow(non_snake_case)]
    pub fn getInstance(context: &OperitApplicationContext) -> Self {
        let database = AppDatabase::getDatabase(RuntimeStorePaths::default())
            .expect("AppDatabase must initialize for WorkspaceService");
        Self {
            chatDao: database.chatDao(),
            fileSystemHost: context
                .fileSystemHost
                .clone()
                .expect("FileSystemHost must be configured for WorkspaceService"),
        }
    }

    #[allow(non_snake_case)]
    pub fn listWorkspaceFiles(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<Vec<WorkspaceFileEntry>, String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let directoryPath = self.resolveWorkspacePath(&workspaceRoot, &relativePath);
        let entries = self
            .fileSystemHost
            .listFiles(&directoryPath)
            .map_err(|error| error.message)?;
        let mut workspaceEntries = entries
            .into_iter()
            .map(|entry| {
                let childRelativePath = joinRelativePath(&relativePath, &entry.name);
                WorkspaceFileEntry {
                    name: entry.name,
                    path: self.resolveWorkspacePath(&workspaceRoot, &childRelativePath),
                    relativePath: childRelativePath,
                    isDirectory: entry.isDirectory,
                    size: entry.size,
                    lastModified: entry.lastModified,
                }
            })
            .collect::<Vec<_>>();
        workspaceEntries.sort_by(|left, right| {
            left.isDirectory
                .cmp(&right.isDirectory)
                .reverse()
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        Ok(workspaceEntries)
    }

    #[allow(non_snake_case)]
    pub fn readWorkspaceTextFile(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<String, String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath);
        self.fileSystemHost
            .readFile(&filePath)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn readWorkspaceFileBytes(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<WorkspaceFileBytes, String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath);
        let bytes = self
            .fileSystemHost
            .readFileBytes(&filePath)
            .map_err(|error| error.message)?;
        Ok(WorkspaceFileBytes {
            base64Content: STANDARD.encode(bytes),
        })
    }

    #[allow(non_snake_case)]
    pub fn writeWorkspaceTextFile(
        &self,
        chatId: String,
        relativePath: String,
        content: String,
    ) -> Result<(), String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath);
        self.fileSystemHost
            .writeFile(&filePath, &content, false)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn openWorkspaceFile(&self, chatId: String, relativePath: String) -> Result<(), String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath);
        self.fileSystemHost
            .openFile(&filePath)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    fn workspaceRoot(&self, chatId: String) -> Result<String, String> {
        let chat = self
            .chatDao
            .getChatById(&chatId)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Chat does not exist: {chatId}"))?;
        chat.workspace
            .map(|workspace| workspace.trim().to_string())
            .filter(|workspace| !workspace.is_empty())
            .ok_or_else(|| format!("Chat has no bound workspace: {chatId}"))
    }

    #[allow(non_snake_case)]
    fn resolveWorkspacePath(&self, workspaceRoot: &str, relativePath: &str) -> String {
        let trimmedRelativePath = normalizeRelativePath(relativePath);
        if trimmedRelativePath.is_empty() {
            return workspaceRoot.to_string();
        }
        PathBuf::from(workspaceRoot)
            .join(trimmedRelativePath)
            .to_string_lossy()
            .to_string()
    }
}

#[allow(non_snake_case)]
fn joinRelativePath(parent: &str, child: &str) -> String {
    let parent = normalizeRelativePath(parent);
    let child = normalizeRelativePath(child);
    if parent.is_empty() {
        child
    } else {
        format!("{parent}/{child}")
    }
}

#[allow(non_snake_case)]
fn normalizeRelativePath(path: &str) -> String {
    path.replace('\\', "/")
        .trim()
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
}
