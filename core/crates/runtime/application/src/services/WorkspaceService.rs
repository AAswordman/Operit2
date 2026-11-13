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
use operit_vfs::VfsPath;
use operit_vfs::VisualFileSystem::VisualFileSystem;

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
pub struct WorkspaceService {
    chatDao: ChatDao,
    workspaceStore: WorkspacePreferenceStore,
    filesystem: VisualFileSystem,
    runtimeStorageHost: Arc<dyn operit_host_api::RuntimeStorageHost>,
}

impl WorkspaceService {
    /// Creates a workspace service from the configured application context.
    #[allow(non_snake_case)]
    pub fn getInstance(context: &HostManager) -> Self {
        let database = AppDatabase::getDatabase(RuntimeStorePaths::default())
            .expect("AppDatabase must initialize for WorkspaceService");
        let runtimeStorageHost = context
            .runtimeStorageHost
            .as_ref()
            .expect("RuntimeStorageHost must be configured for WorkspaceService");
        Self {
            chatDao: database.chatDao(),
            workspaceStore: WorkspacePreferenceStore::getInstance(),
            filesystem: context.vfs().expect("VFS must be configured for WorkspaceService"),
            runtimeStorageHost: runtimeStorageHost.clone(),
        }
    }

    /// Lists files under a chat-bound workspace relative path.
    #[allow(non_snake_case)]
    pub fn listWorkspaceFiles(
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
        let vfs = self.vfs();
        let entries = vfs.listFiles(&directoryPath)?;
        let mut workspaceEntries = Vec::new();
        for entry in entries {
            let childRelativePath = joinRelativePath(&relativePath, &entry.name)?;
            let path = VfsPath::joinVfsPath(&directoryPath, &entry.name)?;
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

    /// Lists ranked workspace file suggestions for an active @ mention.
    #[allow(non_snake_case)]
    pub fn listMentionWorkspaceSuggestions(
        &self,
        chatId: String,
        searchQuery: String,
    ) -> Result<Vec<WorkspaceFileEntry>, String> {
        let workspace = self.boundWorkspace(&chatId)?
            .ok_or_else(|| format!("Chat has no bound workspace: {chatId}"))?;
        let vfs = self.vfs();
        let normalizedQuery = searchQuery.trim().to_ascii_lowercase();
        let recursive = !normalizedQuery.is_empty();
        let mut suggestions = Vec::<MentionWorkspaceSuggestion>::new();
        for folder in &workspace.folders {
            let gitignoreRules = workspaceGitignoreRules(&vfs, &folder.path)?;
            collectMentionWorkspaceSuggestions(
                &vfs,
                &folder.path,
                &folder.name,
                &normalizedQuery,
                recursive,
                &gitignoreRules,
                &mut suggestions,
            )?;
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
    pub fn listWorkspaceBindingDirectories(
        &self,
        path: String,
    ) -> Result<Vec<WorkspaceFileEntry>, String> {
        let directoryPath = VfsPath::normalizeVfsPath(&path)?;
        let vfs = self.vfs();
        let entries = vfs.listFiles(&directoryPath)?;
        let mut directoryEntries = Vec::new();
        for entry in entries {
            if !entry.isDirectory {
                continue;
            }
            let childPath = VfsPath::joinVfsPath(&directoryPath, &entry.name)?;
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
    pub fn mutateWorkspaceFile(
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
        let vfs = self.vfs();
        let info = vfs.fileExists(&source)?;
        if !info.exists {
            return Err("File or folder no longer exists".to_string());
        }
        if operation == "delete" {
            return self.trackWorkspaceChange(&source, || vfs.deleteFile(&source, info.isDirectory));
        }
        let destinationRelative = VfsPath::normalizeRelativePath(
            destinationPath.as_deref().ok_or("Destination path is required")?,
        )?;
        validateMutableEntryPath(&destinationRelative, workspace.is_some(), true)?;
        let destination = self.filePathForChat(&chatId, &destinationRelative)?;
        vfs.validateTransfer(&source, &destination, info.isDirectory)?;
        if vfs.fileExists(&destination)?.exists {
            return Err("A file or folder with this name already exists".to_string());
        }
        self.trackWorkspaceChanges(&[&source, &destination], || {
            if operation == "copy" {
                vfs.copyFile(&source, &destination, info.isDirectory)
            } else {
                vfs.moveFile(&source, &destination)
            }
        })
    }

    /// Reads a text file from a chat-bound workspace.
    #[allow(non_snake_case)]
    pub fn readWorkspaceTextFile(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<String, String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        self.vfs().readFile(&filePath)
    }

    /// Reads a binary file from a chat-bound workspace as base64.
    #[allow(non_snake_case)]
    pub fn readWorkspaceFileBytes(
        &self,
        chatId: String,
        relativePath: String,
    ) -> Result<WorkspaceFileBytes, String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        let bytes = self.vfs().readFileBytes(&filePath)?;
        Ok(WorkspaceFileBytes {
            base64Content: STANDARD.encode(bytes),
        })
    }

    /// Writes a text file into a chat-bound workspace.
    #[allow(non_snake_case)]
    pub fn writeWorkspaceTextFile(
        &self,
        chatId: String,
        relativePath: String,
        content: String,
    ) -> Result<(), String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        self.trackWorkspaceChange(&filePath, || self.vfs()
            .writeFile(&filePath, &content, false))
    }

    /// Writes base64-decoded bytes into a chat-bound workspace file.
    #[allow(non_snake_case)]
    pub fn writeWorkspaceFileBytes(
        &self,
        chatId: String,
        relativePath: String,
        base64Content: String,
    ) -> Result<(), String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        let bytes = STANDARD
            .decode(base64Content.as_bytes())
            .map_err(|error| error.to_string())?;
        self.trackWorkspaceChange(&filePath, || self.vfs()
            .writeFileBytes(&filePath, &bytes))
    }

    /// Opens a chat-bound workspace file through the host file opener.
    #[allow(non_snake_case)]
    pub fn openWorkspaceFile(&self, chatId: String, relativePath: String) -> Result<(), String> {
        let filePath = self.previewFilePathForChat(&chatId, &relativePath)?;
        self.vfs().openFile(&filePath)
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
                let Some(relativePath) =
                    VfsPath::relativePath(VfsPath::workspaceCollectionPath(), &folder.path)?
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

    fn trackWorkspaceChange<T>(&self, path: &str, change: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        self.trackWorkspaceChanges(&[path], change)
    }

    fn trackWorkspaceChanges<T>(&self, paths: &[&str], change: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let mut tracked = Vec::new();
        for path in paths {
            let relative = VfsPath::relativePath(VfsPath::workspaceCollectionPath(), path)?;
            if let Some(relative) = relative.filter(|relative| !relative.is_empty()) {
                tracked.push(format!("workspaces/{relative}"));
            } else if let Some(relative) = path.strip_prefix("/operitapp/outputs/")
                .or_else(|| path.strip_prefix("/app/outputs/")) {
                tracked.push(format!("{}/{relative}", operit_util::RuntimeStorageLayout::RUNTIME_CHAT_OUTPUTS_DIR_PATH));
            }
        }
        if tracked.is_empty() {
            change()
        } else {
            operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(
                self.runtimeStorageHost.clone(), operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
            ).trackPaths(&tracked, change)
        }
    }

    /// Creates a VFS instance scoped to the configured workspace roots.
    #[allow(non_snake_case)]
    fn vfs(&self) -> VisualFileSystem {
        self.filesystem.clone()
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
    VfsPath::joinVfsPath(&folder.path, rest)
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

/// Collects workspace mention candidates from one directory.
#[allow(non_snake_case)]
fn collectMentionWorkspaceSuggestions(
    vfs: &VisualFileSystem,
    directoryVfsPath: &str,
    relativePath: &str,
    normalizedQuery: &str,
    recursive: bool,
    gitignoreRules: &[String],
    suggestions: &mut Vec<MentionWorkspaceSuggestion>,
) -> Result<(), String> {
    for entry in vfs.listFiles(directoryVfsPath)? {
        let childRelativePath = joinRelativePath(relativePath, &entry.name)?;
        if GitIgnoreFilter::shouldIgnore(
            &childRelativePath,
            &entry.name,
            entry.isDirectory,
            gitignoreRules,
        ) {
            continue;
        }

        let childPath = VfsPath::joinVfsPath(directoryVfsPath, &entry.name)?;
        let parentPath = parentRelativePath(&childRelativePath);
        let score = scoreMentionWorkspaceEntry(
            &childRelativePath,
            &entry.name,
            &parentPath,
            entry.isDirectory,
            normalizedQuery,
        );
        if let Some(score) = score {
            if entry.isDirectory || isTextBasedFileName(&entry.name) {
                suggestions.push(MentionWorkspaceSuggestion {
                    entry: WorkspaceFileEntry {
                        name: entry.name.clone(),
                        path: childPath.clone(),
                        relativePath: childRelativePath.clone(),
                        isDirectory: entry.isDirectory,
                        size: entry.size,
                        lastModified: entry.lastModified.clone(),
                    },
                    score,
                });
            }
        }

        if recursive && entry.isDirectory {
            collectMentionWorkspaceSuggestions(
                vfs,
                &childPath,
                &childRelativePath,
                normalizedQuery,
                recursive,
                gitignoreRules,
                suggestions,
            )?;
        }
    }
    Ok(())
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

/// Loads workspace gitignore rules used by mention suggestions.
#[allow(non_snake_case)]
fn workspaceGitignoreRules(
    vfs: &VisualFileSystem,
    workspaceRoot: &str,
) -> Result<Vec<String>, String> {
    let gitignorePath = VfsPath::joinVfsPath(workspaceRoot, ".gitignore")?;
    let gitignoreInfo = vfs.fileExists(&gitignorePath)?;
    if gitignoreInfo.exists && !gitignoreInfo.isDirectory {
        let content = vfs.readFile(&gitignorePath)?;
        return Ok(GitIgnoreFilter::buildRulesFromContent(&content));
    }
    Ok(GitIgnoreFilter::defaultRules())
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
