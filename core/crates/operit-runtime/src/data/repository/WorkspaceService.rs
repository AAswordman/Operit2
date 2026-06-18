use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::FileSystemHost;
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_store::RuntimeStorageLayout::WORKSPACE_DIR_PATH;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::files::PathMapper::PathMapper;
use crate::core::files::VisualFileSystem::VisualFileSystem;
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManagementEntry {
    pub name: String,
    pub fullPath: String,
    pub size: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManagementSummary {
    pub chatHistoryCount: i32,
    pub boundChatCount: i32,
    pub workspaceRoot: String,
    pub unboundWorkspaces: Vec<WorkspaceManagementEntry>,
}

pub struct WorkspaceService {
    chatDao: ChatDao,
    fileSystemHost: Arc<dyn FileSystemHost>,
    runtimeStoreRoot: std::path::PathBuf,
    appFilesRoot: Option<std::path::PathBuf>,
    workspaceCollectionRoot: std::path::PathBuf,
}

impl WorkspaceService {
    #[allow(non_snake_case)]
    pub fn getInstance(context: &OperitApplicationContext) -> Self {
        let database = AppDatabase::getDatabase(RuntimeStorePaths::default())
            .expect("AppDatabase must initialize for WorkspaceService");
        let runtimeStoreRoot = context
            .runtimeStorageHost
            .as_ref()
            .and_then(|host| host.rootDir())
            .expect("RuntimeStorageHost root must be configured for WorkspaceService");
        let workspaceCollectionRoot =
            RuntimeStorePaths::new(runtimeStoreRoot.clone()).workspace_dir();
        Self {
            chatDao: database.chatDao(),
            fileSystemHost: context
                .fileSystemHost
                .clone()
                .expect("FileSystemHost must be configured for WorkspaceService"),
            runtimeStoreRoot,
            appFilesRoot: context.appFilesRoot.clone(),
            workspaceCollectionRoot,
        }
    }

    #[allow(non_snake_case)]
    pub fn listWorkspaceFiles(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<Vec<WorkspaceFileEntry>, String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let directoryPath = self.resolveWorkspacePath(&workspaceRoot, &relativePath)?;
        let vfs = self.vfsForWorkspace(&workspaceRoot);
        let entries = vfs.listFiles(&directoryPath)?;
        let mut workspaceEntries = Vec::new();
        for entry in entries {
            let childRelativePath = joinRelativePath(&relativePath, &entry.name)?;
            let path = self.resolveWorkspacePath(&workspaceRoot, &childRelativePath)?;
            workspaceEntries.push(WorkspaceFileEntry {
                name: entry.name,
                path,
                relativePath: childRelativePath,
                isDirectory: entry.isDirectory,
                size: entry.size,
                lastModified: entry.lastModified,
            });
        }
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
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath)?;
        self.vfsForWorkspace(&workspaceRoot).readFile(&filePath)
    }

    #[allow(non_snake_case)]
    pub fn readWorkspaceFileBytes(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<WorkspaceFileBytes, String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath)?;
        let bytes = self
            .vfsForWorkspace(&workspaceRoot)
            .readFileBytes(&filePath)?;
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
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath)?;
        self.vfsForWorkspace(&workspaceRoot)
            .writeFile(&filePath, &content, false)
    }

    #[allow(non_snake_case)]
    pub fn writeWorkspaceFileBytes(
        &self,
        chatId: String,
        relativePath: String,
        base64Content: String,
    ) -> Result<(), String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath)?;
        let bytes = STANDARD
            .decode(base64Content.as_bytes())
            .map_err(|error| error.to_string())?;
        self.vfsForWorkspace(&workspaceRoot)
            .writeFileBytes(&filePath, &bytes)
    }

    #[allow(non_snake_case)]
    pub fn openWorkspaceFile(&self, chatId: String, relativePath: String) -> Result<(), String> {
        let workspaceRoot = self.workspaceRoot(chatId)?;
        let filePath = self.resolveWorkspacePath(&workspaceRoot, &relativePath)?;
        self.vfsForWorkspace(&workspaceRoot).openFile(&filePath)
    }

    #[allow(non_snake_case)]
    pub fn workspaceManagementSummary(&self) -> Result<WorkspaceManagementSummary, String> {
        let chats = self
            .chatDao
            .getAllChatsDirectly()
            .map_err(|error| error.to_string())?;
        let workspaceRootText = self.workspaceCollectionRoot.to_string_lossy().to_string();
        let mut boundWorkspaceNames = std::collections::HashSet::new();
        let mut boundChatCount = 0i32;

        for chat in &chats {
            let Some(workspace) = chat.workspace.as_ref() else {
                continue;
            };
            let workspace = workspace.trim();
            if workspace.is_empty() {
                continue;
            }
            boundChatCount += 1;
            let Some(relativePath) =
                PathMapper::relativePath(PathMapper::workspaceCollectionPath(), workspace)?
            else {
                continue;
            };
            let components = relativePath.split('/').collect::<Vec<_>>();
            if components.len() != 1 || components[0].is_empty() {
                continue;
            }
            boundWorkspaceNames.insert(components[0].to_string());
        }

        let mut unboundWorkspaces = Vec::new();
        for entry in defaultRuntimeStorageHost()
            .list(WORKSPACE_DIR_PATH)
            .map_err(|error| error.to_string())?
        {
            if !entry.isDirectory {
                continue;
            }
            let name = workspaceNameFromRuntimeStoragePath(&entry.path)?;
            if boundWorkspaceNames.contains(&name) {
                continue;
            }
            unboundWorkspaces.push(WorkspaceManagementEntry {
                fullPath: PathMapper::workspacePath(&name)?,
                name,
                size: entry.size,
            });
        }
        unboundWorkspaces.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(WorkspaceManagementSummary {
            chatHistoryCount: chats.len() as i32,
            boundChatCount,
            workspaceRoot: workspaceRootText,
            unboundWorkspaces,
        })
    }

    #[allow(non_snake_case)]
    pub fn deleteUnboundWorkspaces(&self, workspaceNames: Vec<String>) -> Result<i32, String> {
        let summary = self.workspaceManagementSummary()?;
        let unboundNames = summary
            .unboundWorkspaces
            .into_iter()
            .map(|workspace| workspace.name)
            .collect::<std::collections::HashSet<_>>();
        let storage = defaultRuntimeStorageHost();
        let mut deletedCount = 0i32;
        for workspaceName in workspaceNames {
            validateWorkspaceName(&workspaceName)?;
            if !unboundNames.contains(&workspaceName) {
                return Err(format!(
                    "workspace is not an unbound runtime workspace: {workspaceName}"
                ));
            }
            storage
                .delete(&format!("{WORKSPACE_DIR_PATH}/{workspaceName}"), true)
                .map_err(|error| error.to_string())?;
            deletedCount += 1;
        }
        Ok(deletedCount)
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
    fn vfsForWorkspace(&self, workspaceRoot: &str) -> VisualFileSystem {
        VisualFileSystem::new(
            self.fileSystemHost.clone(),
            PathMapper::new(
                self.runtimeStoreRoot.clone(),
                self.appFilesRoot.clone(),
                self.workspaceCollectionRoot.clone(),
            ),
        )
    }

    #[allow(non_snake_case)]
    fn resolveWorkspacePath(
        &self,
        workspaceRoot: &str,
        relativePath: &str,
    ) -> Result<String, String> {
        PathMapper::joinVfsPath(workspaceRoot, relativePath)
    }
}

#[allow(non_snake_case)]
fn joinRelativePath(parent: &str, child: &str) -> Result<String, String> {
    let parent = PathMapper::normalizeRelativePath(parent)?;
    let child = PathMapper::normalizeRelativePath(child)?;
    if parent.is_empty() {
        Ok(child)
    } else {
        Ok(format!("{parent}/{child}"))
    }
}

#[allow(non_snake_case)]
fn workspaceNameFromRuntimeStoragePath(path: &str) -> Result<String, String> {
    let prefix = format!("{WORKSPACE_DIR_PATH}/");
    let relative = path
        .strip_prefix(&prefix)
        .ok_or_else(|| format!("runtime workspace entry is outside workspace root: {path}"))?;
    validateWorkspaceName(relative)?;
    Ok(relative.to_string())
}

#[allow(non_snake_case)]
fn validateWorkspaceName(workspaceName: &str) -> Result<(), String> {
    let trimmed = workspaceName.trim();
    if trimmed.is_empty() {
        return Err("workspace name is required".to_string());
    }
    if trimmed != workspaceName {
        return Err(format!("invalid workspace name: {workspaceName}"));
    }
    let mut segments = trimmed.split('/');
    let first = segments
        .next()
        .ok_or_else(|| "workspace name is required".to_string())?;
    if segments.next().is_some() {
        return Err(format!("invalid workspace name: {workspaceName}"));
    }
    if first == "." || first == ".." {
        return Err(format!("invalid workspace name: {workspaceName}"));
    }
    if first.chars().any(|character| character == '\\') {
        return Err(format!("invalid workspace name: {workspaceName}"));
    }
    Ok(())
}
