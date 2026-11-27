use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStorageLayout::WORKSPACE_DIR_PATH;
use serde::{Deserialize, Serialize};

use crate::ui::features::chat::webview::workspace::process::GitIgnoreFilter::GitIgnoreFilter;
use operit_host_api::HostManager::HostManager;
use operit_model::Workspace::Workspace;
use operit_store::dao::ChatDao::ChatDao;
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::repository::WorkspacePreferenceStore::WorkspacePreferenceStore;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolResultDataClasses::{FileExistsData, FileEntry, ToolResultData};
use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
use operit_util::FileUtils::FileUtils;
use operit_vfs::VfsPath;

const MAX_MENTION_FILE_SUGGESTIONS: usize = 10;

/// File metadata returned when browsing a chat workspace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileEntry {
    pub name: String,
    pub path: String,
    pub relativePath: String,
    pub isDirectory: bool,
    pub size: i64,
    pub lastModified: String,
}

/// Base64-encoded file bytes returned through the workspace bridge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileBytes {
    pub base64Content: String,
}

/// Runtime workspace entry shown in workspace management views.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManagementEntry {
    pub name: String,
    pub fullPath: String,
    pub size: i64,
}

/// Aggregate workspace-management state for bound and unbound workspaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManagementSummary {
    pub chatHistoryCount: i32,
    pub boundChatCount: i32,
    pub workspaceRoot: String,
    pub unboundWorkspaces: Vec<WorkspaceManagementEntry>,
}

/// Provides chat-bound workspace file operations through the virtual file system.
#[derive(Clone)]
pub struct WorkspaceService {
    chatDao: ChatDao,
    workspaceStore: WorkspacePreferenceStore,
    toolHandler: AIToolHandler,
    runtimeStorageHost: Arc<dyn operit_host_api::RuntimeStorageHost>,
    taskSchedulerHost: Option<Arc<dyn operit_host_api::HostRuntimeTaskSchedulerHost>>,
}

impl WorkspaceService {
    /// Creates a workspace service from the configured application context.
    #[allow(non_snake_case)]
    pub fn new(context: &HostManager, toolHandler: AIToolHandler) -> Self {
        let database = AppDatabase::getDatabase(RuntimeStorePaths::default())
            .expect("AppDatabase must initialize for WorkspaceService");
        let runtimeStorageHost = context
            .runtimeStorageHost
            .as_ref()
            .expect("RuntimeStorageHost must be configured for WorkspaceService");
        Self {
            chatDao: database.chatDao(),
            workspaceStore: WorkspacePreferenceStore::getInstance(),
            toolHandler,
            runtimeStorageHost: runtimeStorageHost.clone(),
            taskSchedulerHost: context.hostRuntimeTaskSchedulerHost.clone(),
        }
    }

    /// Lists files under a chat-bound workspace relative path.
    #[allow(non_snake_case)]
    pub async fn listWorkspaceFiles(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<Vec<WorkspaceFileEntry>, String> {
        let workspace = self.boundWorkspace(&chatId)?;
        let relativePath = VfsPath::normalizeRelativePath(&relativePath)?;
        if let Some(workspace) = workspace.as_ref().filter(|_| relativePath.is_empty()) {
            let mut workspaceEntries = workspace
                .folders
                .iter()
                .map(|folder| WorkspaceFileEntry {
                    name: folder.name.clone(),
                    path: folder.path.clone(),
                    relativePath: folder.name.clone(),
                    isDirectory: true,
                    size: 0,
                    lastModified: String::new(),
                })
                .collect::<Vec<_>>();
            workspaceEntries
                .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
            return Ok(workspaceEntries);
        }
        let directoryPath = self.filePathForChat(&chatId, &relativePath)?;
        let entries = self.listFiles(&directoryPath).await?;
        let mut workspaceEntries = Vec::new();
        for entry in entries {
            let childRelativePath = joinRelativePath(&relativePath, &entry.name)?;
            let path = joinFilePath(&directoryPath, &entry.name)?;
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

    /// Lists ranked file suggestions from the bound workspace or the chat output directory.
    #[allow(non_snake_case)]
    pub async fn listMentionWorkspaceSuggestions(
        &self,
        chatId: String,
        searchQuery: String,
    ) -> Result<Vec<WorkspaceFileEntry>, String> {
        let roots = match self.boundWorkspace(&chatId)? {
            Some(workspace) => workspace.folders.into_iter()
                .map(|folder| (folder.path, folder.name)).collect::<Vec<_>>(),
            None => vec![(self.filePathForChat(&chatId, "")?, String::new())],
        };
        let normalizedQuery = searchQuery.trim().to_ascii_lowercase();
        let recursive = !normalizedQuery.is_empty();
        let mut suggestions = Vec::<MentionWorkspaceSuggestion>::new();
        for (root, relativeRoot) in roots {
            let gitignorePath = joinFilePath(&root, ".gitignore")?;
            let info = self.fileExists(&gitignorePath).await?;
            let rules = if info.exists && !info.isDirectory {
                GitIgnoreFilter::buildRulesFromContent(&self.readText(&gitignorePath).await?)
            } else {
                GitIgnoreFilter::defaultRules()
            };
            let mut directories = vec![(root, relativeRoot)];
            while let Some((directory, relative)) = directories.pop() {
                for entry in self.listFiles(&directory).await? {
                    let childRelative = joinRelativePath(&relative, &entry.name)?;
                    if GitIgnoreFilter::shouldIgnore(&childRelative, &entry.name, entry.isDirectory, &rules) {
                        continue;
                    }
                    let childPath = joinFilePath(&directory, &entry.name)?;
                    if let Some(score) = scoreMentionWorkspaceEntry(
                        &childRelative, &entry.name, &parentRelativePath(&childRelative), entry.isDirectory, &normalizedQuery,
                    ).filter(|_| entry.isDirectory || isTextBasedFileName(&entry.name)) {
                        suggestions.push(MentionWorkspaceSuggestion {
                            score,
                            entry: WorkspaceFileEntry {
                                name: entry.name, path: childPath.clone(), relativePath: childRelative.clone(),
                                isDirectory: entry.isDirectory, size: entry.size, lastModified: entry.lastModified,
                            },
                        });
                    }
                    if recursive && entry.isDirectory {
                        directories.push((childPath, childRelative));
                    }
                }
            }
        }
        suggestions.sort_by(|left, right| {
            left.score.cmp(&right.score).then_with(|| {
                left.entry
                    .relativePath
                    .to_ascii_lowercase()
                    .cmp(&right.entry.relativePath.to_ascii_lowercase())
            })
        });
        Ok(suggestions
            .into_iter()
            .take(MAX_MENTION_FILE_SUGGESTIONS)
            .map(|suggestion| suggestion.entry)
            .collect())
    }

    /// Lists directories that can be selected as workspace binding targets.
    #[allow(non_snake_case)]
    pub async fn listWorkspaceBindingDirectories(
        &self,
        path: String,
    ) -> Result<Vec<WorkspaceFileEntry>, String> {
        let directoryPath = match FileUtils::splitQualifiedVfsPath(&path) {
            Some((node, path)) => FileUtils::qualifyVfsPath(node, &VfsPath::normalizeVfsPath(path)?),
            None => FileUtils::qualifyVfsPath(
                &operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(self.runtimeStorageHost.clone()).identity()?.nodeId,
                &VfsPath::normalizeVfsPath(&path)?,
            ),
        };
        let entries = self.listFiles(&directoryPath).await?;
        let mut directoryEntries = Vec::new();
        for entry in entries {
            if !entry.isDirectory {
                continue;
            }
            let childPath = joinFilePath(&directoryPath, &entry.name)?;
            directoryEntries.push(WorkspaceFileEntry {
                name: entry.name,
                path: childPath.clone(),
                relativePath: childPath,
                isDirectory: true,
                size: entry.size,
                lastModified: entry.lastModified,
            });
        }
        directoryEntries
            .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        Ok(directoryEntries)
    }

    /// Lists every persisted workspace independently from chat history.
    #[allow(non_snake_case)]
    pub fn listWorkspaces(&self) -> Result<Vec<Workspace>, String> {
        self.workspaceStore.getAll().map_err(|error| error.to_string())
    }

    /// Deletes, copies or moves an entry within the chat's workspace roots.
    #[allow(non_snake_case)]
    pub async fn mutateWorkspaceFile(
        &self,
        chatId: String,
        operation: String,
        relativePath: String,
        destinationPath: Option<String>,
    ) -> Result<(), String> {
        if !matches!(operation.as_str(), "delete" | "copy" | "move") {
            return Err("Unsupported workspace file operation".to_string());
        }
        let relativePath = VfsPath::normalizeRelativePath(&relativePath)?;
        let workspace = self.boundWorkspace(&chatId)?;
        validateMutableEntryPath(&relativePath, workspace.is_some(), operation != "copy")?;
        let source = self.filePathForChat(&chatId, &relativePath)?;
        let info = self.fileExists(&source).await?;
        if !info.exists {
            return Err("File or folder no longer exists".to_string());
        }
        if operation == "delete" {
            return self.fileTool("delete_file", &[("path", &source), ("recursive", &info.isDirectory.to_string())]).await.map(|_| ());
        }
        let destinationRelative = VfsPath::normalizeRelativePath(
            destinationPath.as_deref().ok_or("Destination path is required")?,
        )?;
        validateMutableEntryPath(&destinationRelative, workspace.is_some(), true)?;
        let destination = self.filePathForChat(&chatId, &destinationRelative)?;
        if self.fileExists(&destination).await?.exists {
            return Err("A file or folder with this name already exists".to_string());
        }
        self.fileTool(&format!("{operation}_file"), &[
            ("source", &source), ("destination", &destination), ("recursive", &info.isDirectory.to_string()),
        ]).await.map(|_| ())
    }

    /// Reads a text file from a chat-bound workspace.
    #[allow(non_snake_case)]
    pub async fn readWorkspaceTextFile(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<String, String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        self.readText(&filePath).await
    }

    /// Reads a binary file from a chat-bound workspace as base64.
    #[allow(non_snake_case)]
    pub async fn readWorkspaceFileBytes(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<WorkspaceFileBytes, String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        let ToolResultData::BinaryFileContentData(bytes) = self.fileTool("read_file_binary", &[("path", &filePath)]).await? else {
            return Err("File tool returned invalid binary content".into());
        };
        Ok(WorkspaceFileBytes {
            base64Content: bytes.contentBase64,
        })
    }

    /// Writes a text file into a chat-bound workspace.
    #[allow(non_snake_case)]
    pub async fn writeWorkspaceTextFile(
        &self,
        chatId: String,
        relativePath: String,
        content: String,
    ) -> Result<(), String> {
        self.writeWorkspaceFileBytes(chatId, relativePath, STANDARD.encode(content.as_bytes())).await
    }

    /// Writes base64-decoded bytes into a chat-bound workspace file.
    #[allow(non_snake_case)]
    pub async fn writeWorkspaceFileBytes(
        &self,
        chatId: String,
        relativePath: String,
        base64Content: String,
    ) -> Result<(), String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        self.fileTool("write_file_binary", &[("path", &filePath), ("base64Content", &base64Content)]).await.map(|_| ())
    }

    /// Captures a file-open operation without retaining the service or its proxy lock.
    #[allow(non_snake_case)]
    pub fn openWorkspaceFile(
        &self,
        chatId: String,
        relativePath: String,
    ) -> impl std::future::Future<Output = Result<(), String>> + 'static {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath);
        let service = self.clone();
        let scheduler = self.taskSchedulerHost.clone();
        async move {
            let filePath = filePath?;
            let scheduler = scheduler.ok_or_else(|| {
                "HostRuntimeTaskSchedulerHost is not registered for workspace file opening"
                    .to_string()
            })?;
            let (sender, receiver) = tokio::sync::oneshot::channel();
            scheduler
                .scheduleHostRuntimeBlockingAsyncTask(
                    "workspace-file-open",
                    Box::new(move || Box::pin(async move {
                        let result = service.fileTool("open_file", &[("path", &filePath)]).await.map(|_| ());
                        let _ = sender.send(result);
                    })),
                )
                .map_err(|error| error.message)?;
            receiver
                .await
                .map_err(|error| format!("workspace file opener task stopped: {error}"))?
        }
    }

    /// Builds the workspace-management summary for chat bindings and stored workspace folders.
    #[allow(non_snake_case)]
    pub fn workspaceManagementSummary(&self) -> Result<WorkspaceManagementSummary, String> {
        let chats = self
            .chatDao
            .getAllChatsDirectly()
            .map_err(|error| error.to_string())?;
        let workspaceRootText = VfsPath::workspaceCollectionPath().to_string();
        let mut boundWorkspaceNames = std::collections::HashSet::new();
        let mut boundChatCount = 0i32;

        for chat in &chats {
            let Some(workspaceId) = chat.workspaceId.as_ref() else {
                continue;
            };
            boundChatCount += 1;
            let Some(workspace) = self
                .workspaceStore
                .getById(workspaceId)
                .map_err(|error| error.to_string())?
            else {
                continue;
            };
            for folder in &workspace.folders {
                let path = match FileUtils::splitQualifiedVfsPath(&folder.path) {
                    Some((owner, path)) => {
                        let local = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(self.runtimeStorageHost.clone()).identity()?;
                        if owner != local.nodeId { continue; }
                        path
                    }
                    None => &folder.path,
                };
                let Some(relativePath) =
                    VfsPath::relativePath(VfsPath::workspaceCollectionPath(), path)?
                else {
                    continue;
                };
                let components = relativePath.split('/').collect::<Vec<_>>();
                if components.len() != 1 || components[0].is_empty() {
                    continue;
                }
                boundWorkspaceNames.insert(components[0].to_string());
            }
        }

        let mut unboundWorkspaces = Vec::new();
        for entry in self.runtimeStorageHost
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
                fullPath: VfsPath::workspacePath(&name)?,
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

    /// Deletes workspace folders that are not bound to any chat.
    #[allow(non_snake_case)]
    pub fn deleteUnboundWorkspaces(&self, workspaceNames: Vec<String>) -> Result<i32, String> {
        let summary = self.workspaceManagementSummary()?;
        let unboundNames = summary
            .unboundWorkspaces
            .into_iter()
            .map(|workspace| workspace.name)
            .collect::<std::collections::HashSet<_>>();
        let storage = self.runtimeStorageHost.clone();
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

    /// Returns the named workspace bound to a chat.
    #[allow(non_snake_case)]
    fn boundWorkspace(&self, chatId: &str) -> Result<Option<Workspace>, String> {
        let chat = self
            .chatDao
            .getChatById(chatId)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Chat does not exist: {chatId}"))?;
        match chat.workspaceId {
            Some(id) => self.workspaceStore.getById(&id).map_err(|error| error.to_string()),
            None => Ok(None),
        }
    }

    /// Accepts resolved Markdown file addresses without changing tree mutation paths.
    #[allow(non_snake_case)]
    fn previewFilePathForChat(&self, chatId: &str, path: &str) -> Result<String, String> {
        if let Some((node, path)) = FileUtils::splitQualifiedVfsPath(path) {
            return Ok(FileUtils::qualifyVfsPath(node, &VfsPath::normalizeVfsPath(path)?));
        }
        if path.trim().starts_with('/') {
            return VfsPath::normalizeVfsPath(path);
        }
        self.filePathForChat(chatId, path)
    }

    /// Reuses the same persisted base as file tools when this chat has no workspace.
    #[allow(non_snake_case)]
    fn filePathForChat(&self, chatId: &str, relativePath: &str) -> Result<String, String> {
        if let Some(workspace) = self.boundWorkspace(chatId)? {
            return resolveWorkspaceRelativePath(&workspace, relativePath);
        }
        let manager = operit_store::repository::ChatHistoryManager::ChatHistoryManager::default()
            .map_err(|error| error.to_string())?;
        let directory = manager.fileDirectoryForChat(chatId).map_err(|error| error.to_string())?;
        VfsPath::joinVfsPath(&directory, relativePath)
    }

    /// Executes through the existing file tools, including node routing and synchronization.
    async fn fileTool(&self, name: &str, parameters: &[(&str, &str)]) -> Result<ToolResultData, String> {
        let result = self.toolHandler.clone().executeTool(AITool {
            name: name.to_owned(),
            parameters: parameters.iter().map(|(name, value)| ToolParameter {
                name: (*name).to_owned(), value: (*value).to_owned(),
            }).collect(),
        }).await;
        match (result.success, result.error) {
            (true, _) => Ok(result.result),
            (false, Some(error)) => Err(error),
            (false, None) => Err("File tool failed without reporting an error".into()),
        }
    }

    /// Reads directory entries from the VFS on the address's owning node.
    async fn listFiles(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        match self.fileTool("list_files", &[("path", path)]).await? {
            ToolResultData::DirectoryListingData(listing) => Ok(listing.entries),
            _ => Err("File tool returned invalid directory entries".into()),
        }
    }

    /// Reads existence metadata without interpreting a foreign mount locally.
    async fn fileExists(&self, path: &str) -> Result<FileExistsData, String> {
        match self.fileTool("file_exists", &[("path", path)]).await? {
            ToolResultData::FileExistsData(info) => Ok(info),
            _ => Err("File tool returned invalid existence metadata".into()),
        }
    }

    /// Reads UTF-8 content through the same binary transport used by file previews.
    async fn readText(&self, path: &str) -> Result<String, String> {
        let ToolResultData::BinaryFileContentData(bytes) = self.fileTool("read_file_binary", &[("path", path)]).await? else {
            return Err("File tool returned invalid binary content".into());
        };
        String::from_utf8(STANDARD.decode(bytes.contentBase64).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    /// Builds a mention on the mounted folder's owner using the same file access as previews.
    pub(crate) async fn workspaceMentionContent(&self, chatId: &str, relativePath: &str) -> Result<(String, String), String> {
        let path = self.filePathForChat(chatId, relativePath)?;
        let info = self.fileExists(&path).await?;
        if !info.exists { return Err("工作区路径不存在".into()); }
        if info.isDirectory {
            let entries = self.listWorkspaceFiles(chatId.to_owned(), relativePath.to_owned()).await?;
            let text = if entries.is_empty() { "(empty)\n".to_owned() } else {
                entries.into_iter().map(|entry| format!("{} {}", if entry.isDirectory { "[DIR]" } else { "[FILE]" }, entry.name))
                    .collect::<Vec<_>>().join("\n")
            };
            Ok(("application/vnd.workspace-directory+plain".into(), format!(
                "Selected workspace directory: {relativePath}\nThis directory was referenced via @ mention.\n\nDirectory entries:\n{text}"
            )))
        } else {
            let text = self.readText(&path).await?;
            Ok(("text/plain".into(), format!(
                "Selected workspace file: {relativePath}\nThis file was referenced via @ mention.\n\nFile content:\n{text}"
            )))
        }
    }
}

/// Joins inside a mounted folder without losing its owning node.
pub(crate) fn joinFilePath(path: &str, relative: &str) -> Result<String, String> {
    match FileUtils::splitQualifiedVfsPath(path) {
        Some((node, path)) => Ok(FileUtils::qualifyVfsPath(node, &VfsPath::joinVfsPath(path, relative)?)),
        None => VfsPath::joinVfsPath(path, relative),
    }
}

/// Resolves a workspace-relative path against one of the mounted folders.
fn resolveWorkspaceRelativePath(
    workspace: &Workspace,
    relativePath: &str,
) -> Result<String, String> {
    let relativePath = VfsPath::normalizeRelativePath(relativePath)?;
    if relativePath.is_empty() {
        return Err("workspace-relative path must include a folder name".to_string());
    }
    let (folderName, rest) = match relativePath.split_once('/') {
        Some((folderName, rest)) => (folderName, rest),
        None => (relativePath.as_str(), ""),
    };
    let folder = workspace
        .folderByName(folderName)
        .ok_or_else(|| format!("workspace folder not found: {folderName}"))?;
    joinFilePath(&folder.path, rest)
}

/// Joins two workspace-relative path segments.
#[allow(non_snake_case)]
fn joinRelativePath(parent: &str, child: &str) -> Result<String, String> {
    let parent = VfsPath::normalizeRelativePath(parent)?;
    let child = VfsPath::normalizeRelativePath(child)?;
    if parent.is_empty() {
        Ok(child)
    } else {
        Ok(format!("{parent}/{child}"))
    }
}

struct MentionWorkspaceSuggestion {
    entry: WorkspaceFileEntry,
    score: i32,
}

/// Scores one workspace entry against the active mention query.
#[allow(non_snake_case)]
fn scoreMentionWorkspaceEntry(
    relativePath: &str,
    displayName: &str,
    parentPath: &str,
    isDirectory: bool,
    query: &str,
) -> Option<i32> {
    let normalizedName = displayName.to_ascii_lowercase();
    let normalizedPath = relativePath.to_ascii_lowercase();
    let normalizedParentPath = parentPath.to_ascii_lowercase();
    let depth = relativePath
        .chars()
        .filter(|character| *character == '/')
        .count() as i32;

    if query.is_empty() {
        return Some(if isDirectory { depth } else { 20 + depth });
    }

    let baseScore = if normalizedName == query {
        0
    } else if normalizedPath == query {
        1
    } else if normalizedName.starts_with(query) {
        2
    } else if normalizedPath.starts_with(query) {
        3
    } else if normalizedParentPath.find(query).is_some() {
        4
    } else if normalizedName.find(query).is_some() {
        5
    } else if normalizedPath.find(query).is_some() {
        6
    } else {
        return None;
    };

    Some(if isDirectory {
        baseScore * 2
    } else {
        baseScore * 2 + 1
    })
}

/// Builds the parent path shown for a workspace mention suggestion.
#[allow(non_snake_case)]
fn parentRelativePath(relativePath: &str) -> String {
    relativePath
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

/// Reports whether a file name is suitable for text mention attachments.
#[allow(non_snake_case)]
fn isTextBasedFileName(fileName: &str) -> bool {
    let lower = fileName.to_ascii_lowercase();
    let extension = lower
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or("");
    matches!(
        extension,
        "txt"
            | "md"
            | "markdown"
            | "rs"
            | "kt"
            | "kts"
            | "java"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "dart"
            | "py"
            | "json"
            | "json5"
            | "toml"
            | "yaml"
            | "yml"
            | "xml"
            | "html"
            | "css"
            | "scss"
            | "gradle"
            | "properties"
            | "ini"
            | "csv"
            | "sh"
            | "bash"
            | "zsh"
            | "ps1"
            | "bat"
            | "cmd"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "go"
            | "swift"
            | "sql"
            | "lock"
    ) || lower.find('.').is_none()
}

/// Extracts a workspace directory name from a runtime storage path.
#[allow(non_snake_case)]
fn workspaceNameFromRuntimeStoragePath(path: &str) -> Result<String, String> {
    let prefix = format!("{WORKSPACE_DIR_PATH}/");
    let relative = path
        .strip_prefix(&prefix)
        .ok_or_else(|| format!("runtime workspace entry is outside workspace root: {path}"))?;
    validateWorkspaceName(relative)?;
    Ok(relative.to_string())
}

/// Validates a single runtime workspace directory name.
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

/// Mounted folder roots are managed through workspace binding, not file actions.
fn validateMutableEntryPath(path: &str, bound: bool, modifying: bool) -> Result<(), String> {
    if path.is_empty() || (bound && modifying && !path.contains('/')) {
        return Err("Workspace roots cannot be renamed, deleted or replaced".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod file_mutation_tests {
    use super::*;

    #[test]
    fn protects_roots_but_allows_files_and_root_copies() {
        assert!(validateMutableEntryPath("", false, true).is_err());
        assert!(validateMutableEntryPath("project", true, true).is_err());
        assert!(validateMutableEntryPath("project", true, false).is_ok());
        assert!(validateMutableEntryPath("project/file.txt", true, true).is_ok());
        assert!(validateMutableEntryPath("file.txt", false, true).is_ok());
    }

}
