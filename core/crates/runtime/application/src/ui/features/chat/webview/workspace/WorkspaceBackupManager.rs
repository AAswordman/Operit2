use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use operit_host_api::FindFilesRequest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ui::features::chat::webview::workspace::process::GitIgnoreFilter::GitIgnoreFilter;
use operit_host_api::HostManager::HostManager;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_tools::tools::AIToolHook::AIToolHook;
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::AITool;

/// Snapshot data belongs to the workspace and is excluded from its own captures.
const BACKUP_DIR_NAME: &str = ".operit/backups";
const OBJECTS_DIR_NAME: &str = "objects";
const STATE_FILE_NAME: &str = "state.json";

const WORKSPACE_MUTATING_TOOLS: [&str; 9] = [
    "apply_file",
    "create_file",
    "edit_file",
    "write_file",
    "write_file_binary",
    "move_file",
    "delete_file",
    "copy_file",
    "make_directory",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFileChange {
    pub path: String,
    pub changeType: ChangeType,
    pub changedLines: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    ADDED,
    DELETED,
    MODIFIED,
}

/// One changed file in the retained turn; `None` means the file was absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct TurnFileSnapshot {
    before: Option<String>,
    after: Option<String>,
}

/// The only snapshot kept for a chat: the most recent turn that touched files.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorkspaceTurnSnapshot {
    turnTimestamp: i64,
    workspacePath: String,
    files: BTreeMap<String, TurnFileSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceRewindOutcome {
    Restored,
    NoRestoreNeeded,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RewindAvailability {
    Unavailable,
    NoRestoreNeeded,
    UseSnapshot,
}

/// Only the latest editing turn can be restored; later turns without edits
/// need no restore, and older turns have already been discarded.
fn rewindAvailability(turnTimestamp: i64, messageTimestamp: i64) -> RewindAvailability {
    if messageTimestamp < turnTimestamp {
        RewindAvailability::Unavailable
    } else if messageTimestamp > turnTimestamp {
        RewindAvailability::NoRestoreNeeded
    } else {
        RewindAvailability::UseSnapshot
    }
}

#[derive(Clone)]
pub struct WorkspaceBackupManager {
    context: HostManager,
}

struct WorkspaceToolHookState {
    initialized: bool,
    beforeFiles: BTreeMap<String, Capture>,
    affectedPaths: BTreeSet<String>,
    gitignoreRules: Vec<String>,
}

pub struct WorkspaceToolHookSession {
    id: String,
    manager: WorkspaceBackupManager,
    workspacePath: String,
    messageTimestamp: i64,
    chatScopeId: Option<String>,
    closed: AtomicBool,
    state: Mutex<WorkspaceToolHookState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Capture {
    Absent,
    Present(String),
    Unknown,
}

impl WorkspaceBackupManager {
    pub fn new(context: HostManager) -> Self {
        Self { context }
    }

    #[allow(non_snake_case)]
    pub fn getInstance(context: HostManager) -> Self {
        Self::new(context)
    }

    #[allow(non_snake_case)]
    pub fn createWorkspaceToolHookSession(
        &self,
        workspacePath: String,
        messageTimestamp: i64,
        chatId: Option<String>,
    ) -> Arc<WorkspaceToolHookSession> {
        Arc::new(WorkspaceToolHookSession {
            id: format!(
                "workspace-backup-{}-{messageTimestamp}",
                normalizeChatScope(chatId.as_deref())
            ),
            manager: self.clone(),
            workspacePath,
            messageTimestamp,
            chatScopeId: chatId,
            closed: AtomicBool::new(false),
            state: Mutex::new(WorkspaceToolHookState {
                initialized: false,
                beforeFiles: BTreeMap::new(),
                affectedPaths: BTreeSet::new(),
                gitignoreRules: Vec::new(),
            }),
        })
    }

    /// Returns `None` when this message's workspace snapshot is no longer kept.
    #[allow(non_snake_case)]
    pub fn previewChangesForMessage(
        &self,
        workspacePath: String,
        messageTimestamp: i64,
        chatId: Option<String>,
    ) -> Option<Vec<WorkspaceFileChange>> {
        let vfs = self.vfs();
        let snapshot = self.loadTurnSnapshot(&vfs, &workspacePath, chatId.as_deref());
        let turnTimestamp = snapshot
            .as_ref()
            .map(|snapshot| snapshot.turnTimestamp)
            .unwrap_or(0);
        match rewindAvailability(turnTimestamp, messageTimestamp) {
            RewindAvailability::Unavailable => None,
            RewindAvailability::NoRestoreNeeded => Some(Vec::new()),
            RewindAvailability::UseSnapshot => {
                let snapshot = snapshot?;
                if snapshot.workspacePath != normalizeWorkspacePath(&workspacePath) {
                    return None;
                }
                Some(self.changesFromSnapshot(&vfs, chatId.as_deref(), &snapshot))
            }
        }
    }

    /// Restores the retained turn, or reports why no workspace restore happened.
    #[allow(non_snake_case)]
    pub fn restoreWorkspaceForMessage(
        &self,
        workspacePath: String,
        messageTimestamp: i64,
        chatId: Option<String>,
    ) -> WorkspaceRewindOutcome {
        let vfs = self.vfs();
        let snapshot = self.loadTurnSnapshot(&vfs, &workspacePath, chatId.as_deref());
        let turnTimestamp = snapshot
            .as_ref()
            .map(|snapshot| snapshot.turnTimestamp)
            .unwrap_or(0);
        match rewindAvailability(turnTimestamp, messageTimestamp) {
            RewindAvailability::Unavailable => return WorkspaceRewindOutcome::Unavailable,
            RewindAvailability::NoRestoreNeeded => {
                return WorkspaceRewindOutcome::NoRestoreNeeded
            }
            RewindAvailability::UseSnapshot => {}
        }
        let Some(snapshot) = snapshot else {
            return WorkspaceRewindOutcome::NoRestoreNeeded;
        };
        if snapshot.workspacePath != normalizeWorkspacePath(&workspacePath) {
            return WorkspaceRewindOutcome::Unavailable;
        }

        let objectsDir = self.objectsDir(&workspacePath, chatId.as_deref());
        let mut filesToWrite = Vec::new();
        let mut filesToDelete = Vec::new();
        for (path, entry) in &snapshot.files {
            let targetPath = joinPath(&workspacePath, path);
            match &entry.before {
                Some(hash) => {
                    let Ok(bytes) = vfs.readFileBytes(&joinPath(&objectsDir, hash)) else {
                        return WorkspaceRewindOutcome::Unavailable;
                    };
                    filesToWrite.push((targetPath, bytes));
                }
                None => filesToDelete.push(targetPath),
            }
        }
        for (targetPath, bytes) in filesToWrite {
            if let Some((parent, _)) = targetPath.rsplit_once('/') {
                if !parent.is_empty() {
                    ensureDirectory(&vfs, parent);
                }
            }
            let _ = vfs.writeFileBytes(&targetPath, &bytes);
        }
        for targetPath in filesToDelete {
            let _ = vfs.deleteFile(&targetPath, false);
        }

        // Keep the timestamp as a watermark so older messages stay unavailable.
        let _ = vfs.deleteFile(&objectsDir, true);
        ensureDirectory(&vfs, &objectsDir);
        self.saveTurnSnapshot(
            &vfs,
            chatId.as_deref(),
            &WorkspaceTurnSnapshot {
                turnTimestamp: snapshot.turnTimestamp,
                workspacePath: snapshot.workspacePath,
                files: BTreeMap::new(),
            },
        );
        WorkspaceRewindOutcome::Restored
    }

    /// Drops all retained snapshot data owned by one chat.
    #[allow(non_snake_case)]
    pub fn deleteChatState(&self, workspacePath: String, chatId: Option<String>) {
        let vfs = self.vfs();
        let _ = vfs.deleteFile(&self.chatBackupDir(&workspacePath, chatId.as_deref()), true);
    }

    fn vfs(&self) -> VisualFileSystem {
        let runtimeStorageHost = self
            .context
            .runtimeStorageHost
            .as_ref()
            .expect("RuntimeStorageHost must be configured for WorkspaceBackupManager");
        let runtimeStoreRoot = runtimeStorageHost.runtimeRootDir().expect(
            "RuntimeStorageHost runtime root must be configured for WorkspaceBackupManager",
        );
        let workspaceCollectionRoot = runtimeStorageHost.workspaceRootDir().expect(
            "RuntimeStorageHost workspace root must be configured for WorkspaceBackupManager",
        );
        VisualFileSystem::new(
            self.context
                .fileSystemHost
                .clone()
                .expect("FileSystemHost must be configured for WorkspaceBackupManager"),
            PathMapper::new(runtimeStoreRoot, workspaceCollectionRoot, runtimeStorageHost.clone()),
        )
    }

    fn chatBackupDir(&self, workspacePath: &str, chatId: Option<&str>) -> String {
        joinPath(
            &joinPath(workspacePath, BACKUP_DIR_NAME),
            &normalizeChatScope(chatId),
        )
    }

    fn objectsDir(&self, workspacePath: &str, chatId: Option<&str>) -> String {
        joinPath(&self.chatBackupDir(workspacePath, chatId), OBJECTS_DIR_NAME)
    }

    fn statePath(&self, workspacePath: &str, chatId: Option<&str>) -> String {
        joinPath(&self.chatBackupDir(workspacePath, chatId), STATE_FILE_NAME)
    }

    fn loadTurnSnapshot(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
        chatId: Option<&str>,
    ) -> Option<WorkspaceTurnSnapshot> {
        let content = vfs.readFile(&self.statePath(workspacePath, chatId)).ok()?;
        if content.trim().is_empty() {
            return None;
        }
        serde_json::from_str(&content).ok()
    }

    fn saveTurnSnapshot(
        &self,
        vfs: &VisualFileSystem,
        chatId: Option<&str>,
        snapshot: &WorkspaceTurnSnapshot,
    ) {
        ensureDirectory(vfs, &self.chatBackupDir(&snapshot.workspacePath, chatId));
        let content =
            serde_json::to_string(snapshot).expect("WorkspaceTurnSnapshot must serialize");
        let _ = vfs.writeFile(
            &self.statePath(&snapshot.workspacePath, chatId),
            &content,
            false,
        );
    }

    /// Starts a new editing turn and discards the previous turn's snapshot.
    fn beginTurn(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
        messageTimestamp: i64,
        chatId: Option<&str>,
    ) -> Option<Vec<String>> {
        let workspaceInfo = vfs.fileExists(workspacePath).ok()?;
        if !workspaceInfo.exists || !workspaceInfo.isDirectory {
            return None;
        }
        let objectsDir = self.objectsDir(workspacePath, chatId);
        let _ = vfs.deleteFile(&objectsDir, true);
        ensureDirectory(vfs, &objectsDir);
        self.saveTurnSnapshot(
            vfs,
            chatId,
            &WorkspaceTurnSnapshot {
                turnTimestamp: messageTimestamp,
                workspacePath: normalizeWorkspacePath(workspacePath),
                files: BTreeMap::new(),
            },
        );
        Some(self.loadGitignoreRulesProvider(vfs, workspacePath))
    }

    /// Captures after-state once, when the turn ends, and persists the changes.
    fn finalizeTurn(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
        messageTimestamp: i64,
        chatId: Option<&str>,
        gitignoreRules: &[String],
        beforeFiles: &BTreeMap<String, Capture>,
        affectedPaths: &BTreeSet<String>,
    ) {
        let objectsDir = self.objectsDir(workspacePath, chatId);
        let mut afterFiles = BTreeMap::new();
        for path in affectedPaths {
            self.capturePath(
                vfs,
                workspacePath,
                path,
                gitignoreRules,
                &objectsDir,
                &mut afterFiles,
            );
        }
        self.saveTurnSnapshot(
            vfs,
            chatId,
            &WorkspaceTurnSnapshot {
                turnTimestamp: messageTimestamp,
                workspacePath: normalizeWorkspacePath(workspacePath),
                files: mergeTurnFiles(beforeFiles, &afterFiles),
            },
        );
    }

    /// Captures one affected path; the first capture of a file wins in a turn.
    fn capturePath(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
        targetPath: &str,
        gitignoreRules: &[String],
        objectsDir: &str,
        captures: &mut BTreeMap<String, Capture>,
    ) {
        let normalizedTargetPath = targetPath.trim().trim_end_matches('/');
        let Some(relativeTarget) = makeRelativePath(workspacePath, normalizedTargetPath) else {
            return;
        };
        let Ok(existsData) = vfs.fileExists(normalizedTargetPath) else {
            return;
        };
        if !existsData.exists {
            return;
        }
        if existsData.isDirectory {
            for childPath in self.listWorkspaceTextFilesUnderPathProvider(
                vfs,
                workspacePath,
                normalizedTargetPath,
                gitignoreRules,
            ) {
                let Some(relativeChildPath) = makeRelativePath(workspacePath, &childPath) else {
                    continue;
                };
                if captures.contains_key(&relativeChildPath) {
                    continue;
                }
                let capture = match self.snapshotFile(vfs, &childPath, objectsDir) {
                    Some(hash) => Capture::Present(hash),
                    None => Capture::Unknown,
                };
                captures.insert(relativeChildPath, capture);
            }
            return;
        }
        if relativeTarget.is_empty() {
            return;
        }
        let fileName = relativeTarget.rsplit('/').next().unwrap_or(&relativeTarget);
        if !isTextBasedFileName(fileName)
            || GitIgnoreFilter::shouldIgnore(&relativeTarget, fileName, false, gitignoreRules)
        {
            return;
        }
        if captures.contains_key(&relativeTarget) {
            return;
        }
        let capture = match self.snapshotFile(vfs, normalizedTargetPath, objectsDir) {
            Some(hash) => Capture::Present(hash),
            None => Capture::Unknown,
        };
        captures.insert(relativeTarget, capture);
    }

    fn listWorkspaceTextFilesUnderPathProvider(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
        startPath: &str,
        gitignoreRules: &[String],
    ) -> Vec<String> {
        let Ok(allFiles) = vfs.findFiles(FindFilesRequest {
            path: startPath.to_string(),
            pattern: "*".to_string(),
            maxDepth: -1,
            usePathPattern: false,
            caseInsensitive: false,
        }) else {
            return Vec::new();
        };

        allFiles
            .into_iter()
            .filter(|fullPath| {
                let Some(relative) = makeRelativePath(workspacePath, fullPath) else {
                    return false;
                };
                if relative.is_empty() {
                    return false;
                }
                let fileName = relative.rsplit('/').next().unwrap_or(relative.as_str());
                isTextBasedFileName(fileName)
                    && !GitIgnoreFilter::shouldIgnore(&relative, fileName, false, gitignoreRules)
            })
            .collect()
    }

    fn snapshotFile(
        &self,
        vfs: &VisualFileSystem,
        filePath: &str,
        objectsDir: &str,
    ) -> Option<String> {
        let bytes = vfs.readFileBytes(filePath).ok()?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let objectPath = joinPath(objectsDir, &hash);
        let objectExists = vfs
            .fileExists(&objectPath)
            .map(|value| value.exists)
            .unwrap_or(false);
        if !objectExists {
            ensureDirectory(vfs, objectsDir);
            if vfs.writeFileBytes(&objectPath, &bytes).is_err() {
                return None;
            }
        }
        Some(hash)
    }

    fn loadGitignoreRulesProvider(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
    ) -> Vec<String> {
        let gitignorePath = joinPath(workspacePath, ".gitignore");
        match vfs.readFile(&gitignorePath) {
            Ok(content) if !content.trim().is_empty() => {
                GitIgnoreFilter::buildRulesFromContent(&content)
            }
            _ => GitIgnoreFilter::defaultRules(),
        }
    }

    fn changesFromSnapshot(
        &self,
        vfs: &VisualFileSystem,
        chatId: Option<&str>,
        snapshot: &WorkspaceTurnSnapshot,
    ) -> Vec<WorkspaceFileChange> {
        let objectsDir = self.objectsDir(&snapshot.workspacePath, chatId);
        let mut changes = Vec::new();
        for (path, entry) in &snapshot.files {
            match (&entry.before, &entry.after) {
                (Some(before), Some(after)) => {
                    let changedLines = self.estimateChangedLinesBetweenHashesProvider(
                        vfs, &objectsDir, after, before,
                    );
                    if changedLines > 0 {
                        changes.push(WorkspaceFileChange {
                            path: path.clone(),
                            changeType: ChangeType::MODIFIED,
                            changedLines,
                        });
                    }
                }
                (Some(before), None) => {
                    changes.push(WorkspaceFileChange {
                        path: path.clone(),
                        changeType: ChangeType::ADDED,
                        changedLines: self.estimateLineCountFromHashProvider(
                            vfs, &objectsDir, before,
                        ),
                    });
                }
                (None, Some(after)) => {
                    changes.push(WorkspaceFileChange {
                        path: path.clone(),
                        changeType: ChangeType::DELETED,
                        changedLines: self.estimateLineCountFromHashProvider(
                            vfs, &objectsDir, after,
                        ),
                    });
                }
                (None, None) => {}
            }
        }
        changes.sort_by(|left, right| left.path.cmp(&right.path));
        changes
    }

    fn readTextFromObjectHashProvider(
        &self,
        vfs: &VisualFileSystem,
        objectsDir: &str,
        hash: &str,
    ) -> Option<String> {
        let bytes = vfs.readFileBytes(&joinPath(objectsDir, hash)).ok()?;
        String::from_utf8(bytes).ok()
    }

    fn estimateLineCountFromHashProvider(
        &self,
        vfs: &VisualFileSystem,
        objectsDir: &str,
        hash: &str,
    ) -> i32 {
        let Some(text) = self.readTextFromObjectHashProvider(vfs, objectsDir, hash) else {
            return 0;
        };
        normalizeTextLinesForDiff(&text).len() as i32
    }

    fn estimateChangedLinesBetweenHashesProvider(
        &self,
        vfs: &VisualFileSystem,
        objectsDir: &str,
        currentHash: &str,
        targetHash: &str,
    ) -> i32 {
        if currentHash == targetHash {
            return 0;
        }
        let Some(currentText) = self.readTextFromObjectHashProvider(vfs, objectsDir, currentHash)
        else {
            return 0;
        };
        let Some(targetText) = self.readTextFromObjectHashProvider(vfs, objectsDir, targetHash)
        else {
            return 0;
        };
        estimateChangedLines(&currentText, &targetText)
    }
}

impl WorkspaceToolHookSession {
    #[allow(non_snake_case)]
    pub fn hookId(&self) -> &str {
        &self.id
    }

    pub fn close(&self) {
        let state = self.state.lock().expect("WorkspaceToolHookSession mutex poisoned");
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        if !state.initialized {
            return;
        }
        let vfs = self.manager.vfs();
        self.manager.finalizeTurn(
            &vfs,
            &self.workspacePath,
            self.messageTimestamp,
            self.chatScopeId.as_deref(),
            &state.gitignoreRules,
            &state.beforeFiles,
            &state.affectedPaths,
        );
    }
}

impl AIToolHook for WorkspaceToolHookSession {
    fn id(&self) -> &str {
        &self.id
    }

    /// Captures before-state immediately before each mutating tool runs.
    fn onToolExecutionStarted(&self, tool: &AITool) {
        if self.closed.load(Ordering::SeqCst) || !isWorkspaceMutatingTool(&tool.name) {
            return;
        }
        let affectedPaths = extractWorkspaceAffectedPaths(tool, &self.workspacePath);
        if affectedPaths.is_empty() {
            return;
        }

        let mut state = self.state.lock().expect("WorkspaceToolHookSession mutex poisoned");
        if self.closed.load(Ordering::SeqCst) {
            return;
        }
        if !state.initialized {
            let vfs = self.manager.vfs();
            let Some(gitignoreRules) = self.manager.beginTurn(
                &vfs,
                &self.workspacePath,
                self.messageTimestamp,
                self.chatScopeId.as_deref(),
            ) else {
                return;
            };
            state.gitignoreRules = gitignoreRules;
            state.initialized = true;
        }

        let vfs = self.manager.vfs();
        let objectsDir = self
            .manager
            .objectsDir(&self.workspacePath, self.chatScopeId.as_deref());
        let gitignoreRules = state.gitignoreRules.clone();
        for path in affectedPaths {
            // A path first touched later in the same turn must keep its original
            // before-state; paths created by an earlier tool have no before blob.
            let shouldCapture = !isCoveredByAffectedPath(&state.affectedPaths, &path);
            state.affectedPaths.insert(path.clone());
            if shouldCapture {
                self.manager.capturePath(
                    &vfs,
                    &self.workspacePath,
                    &path,
                    &gitignoreRules,
                    &objectsDir,
                    &mut state.beforeFiles,
                );
            }
        }
    }

    fn onToolExecutionResult(&self, tool: &AITool, result: &ToolResult) {
        if self.closed.load(Ordering::SeqCst)
            || !result.success
            || !isWorkspaceMutatingTool(&tool.name)
        {
            return;
        }
        let affectedPaths = extractWorkspaceAffectedPaths(tool, &self.workspacePath);
        if affectedPaths.is_empty() {
            return;
        }

        let mut state = self.state.lock().expect("WorkspaceToolHookSession mutex poisoned");
        if self.closed.load(Ordering::SeqCst) || !state.initialized {
            return;
        }
        for path in affectedPaths {
            state.affectedPaths.insert(path);
        }
    }
}

fn isWorkspaceMutatingTool(toolName: &str) -> bool {
    WORKSPACE_MUTATING_TOOLS.contains(&toolName)
}

/// Reports whether this path or one of its ancestors was already touched in the
/// current turn, so a later tool cannot overwrite the original before-state.
fn isCoveredByAffectedPath(affectedPaths: &BTreeSet<String>, path: &str) -> bool {
    if affectedPaths.contains(path) {
        return true;
    }
    let mut current = path;
    while let Some((parent, _)) = current.rsplit_once('/') {
        if affectedPaths.contains(parent) {
            return true;
        }
        current = parent;
    }
    false
}

fn extractWorkspaceAffectedPaths(tool: &AITool, workspacePath: &str) -> Vec<String> {
    let mut result = Vec::<String>::new();
    match tool.name.as_str() {
        "apply_file" | "create_file" | "edit_file" | "delete_file" | "write_file"
        | "write_file_binary" | "make_directory" => {
            collectWorkspacePath(&mut result, toolParam(tool, "path"), workspacePath);
        }
        "move_file" => {
            collectWorkspacePath(&mut result, toolParam(tool, "source"), workspacePath);
            collectWorkspacePath(&mut result, toolParam(tool, "destination"), workspacePath);
        }
        "copy_file" => {
            collectWorkspacePath(&mut result, toolParam(tool, "source"), workspacePath);
            collectWorkspacePath(&mut result, toolParam(tool, "destination"), workspacePath);
        }
        _ => {}
    }
    result
}

fn toolParam<'a>(tool: &'a AITool, name: &str) -> Option<&'a str> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.as_str())
}

fn collectWorkspacePath(result: &mut Vec<String>, path: Option<&str>, workspacePath: &str) {
    let Some(rawPath) = path else {
        return;
    };
    let normalizedPath = rawPath.trim().trim_end_matches('/').to_string();
    if normalizedPath.is_empty() {
        return;
    }
    let normalizedPath = if makeRelativePath(workspacePath, &normalizedPath).is_none()
        && !startsWithAbsoluteRoot(&normalizedPath)
    {
        joinPath(workspacePath, &normalizedPath)
    } else {
        normalizedPath
    };
    if makeRelativePath(workspacePath, &normalizedPath).is_some() {
        result.push(normalizedPath);
    }
}

fn startsWithAbsoluteRoot(path: &str) -> bool {
    let normalized = GitIgnoreFilter::normalizePath(path);
    normalized.starts_with('/') || normalized.as_bytes().get(1) == Some(&b':')
}

fn ensureDirectory(vfs: &VisualFileSystem, path: &str) {
    let _ = vfs.makeDirectory(path, true);
}

fn normalizeChatScope(chatId: Option<&str>) -> String {
    let raw = chatId.unwrap_or("").trim();
    if raw.is_empty() {
        return "__default__".to_string();
    }
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn normalizeWorkspacePath(path: &str) -> String {
    GitIgnoreFilter::normalizePath(path)
        .trim_end_matches('/')
        .to_string()
}

fn joinPath(parent: &str, child: &str) -> String {
    let parent = GitIgnoreFilter::normalizePath(parent);
    let child = GitIgnoreFilter::normalizePath(child)
        .trim_start_matches('/')
        .to_string();
    if parent.is_empty() {
        format!("/{child}")
    } else if parent == "/" {
        format!("/{child}")
    } else {
        format!("{}/{}", parent.trim_end_matches('/'), child)
    }
}

fn makeRelativePath(root: &str, fullPath: &str) -> Option<String> {
    let normalizedRoot = normalizeWorkspacePath(root);
    if normalizedRoot.is_empty() {
        return None;
    }
    let normalizedFullPath = GitIgnoreFilter::normalizePath(fullPath);
    makeRelativePathForNormalizedRoot(&normalizedRoot, &normalizedFullPath)
}

fn makeRelativePathForNormalizedRoot(root: &str, fullPath: &str) -> Option<String> {
    if fullPath == root {
        return Some(String::new());
    }
    let prefix = format!("{root}/");
    if !fullPath.starts_with(&prefix) {
        return None;
    }
    Some(fullPath[prefix.len()..].trim_start_matches('/').to_string())
}

/// Keeps only real changes and never restores files whose capture failed.
fn mergeTurnFiles(
    beforeFiles: &BTreeMap<String, Capture>,
    afterFiles: &BTreeMap<String, Capture>,
) -> BTreeMap<String, TurnFileSnapshot> {
    let mut files = BTreeMap::new();
    for path in beforeFiles
        .keys()
        .chain(afterFiles.keys())
        .collect::<BTreeSet<_>>()
    {
        let before = beforeFiles.get(path).cloned().unwrap_or(Capture::Absent);
        let after = afterFiles.get(path).cloned().unwrap_or(Capture::Absent);
        if before == after {
            continue;
        }
        let (before, after) = match (before, after) {
            (Capture::Present(before), Capture::Present(after)) => (Some(before), Some(after)),
            (Capture::Present(before), Capture::Absent) => (Some(before), None),
            (Capture::Absent, Capture::Present(after)) => (None, Some(after)),
            _ => continue,
        };
        files.insert(path.clone(), TurnFileSnapshot { before, after });
    }
    files
}

fn normalizeTextLinesForDiff(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(|line| line.to_string())
        .collect()
}

fn estimateChangedLines(beforeText: &str, afterText: &str) -> i32 {
    if beforeText == afterText {
        return 0;
    }
    let beforeLines = normalizeTextLinesForDiff(beforeText);
    let afterLines = normalizeTextLinesForDiff(afterText);
    let common = longestCommonSubsequenceLength(&beforeLines, &afterLines);
    let deleted = beforeLines.len().saturating_sub(common);
    let inserted = afterLines.len().saturating_sub(common);
    deleted.max(inserted) as i32
}

fn longestCommonSubsequenceLength(left: &[String], right: &[String]) -> usize {
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let mut previous = vec![0usize; right.len() + 1];
    let mut current = vec![0usize; right.len() + 1];
    for leftLine in left {
        for (rightIndex, rightLine) in right.iter().enumerate() {
            current[rightIndex + 1] = if leftLine == rightLine {
                previous[rightIndex] + 1
            } else {
                previous[rightIndex + 1].max(current[rightIndex])
            };
        }
        std::mem::swap(&mut previous, &mut current);
        current.fill(0);
    }
    previous[right.len()]
}

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
    ) || !lower.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_native_filesystem::PosixFileSystemHost;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use operit_tools::ToolExecutionManager::ToolParameter;

    struct BackupFixture {
        root: std::path::PathBuf,
        manager: WorkspaceBackupManager,
    }

    impl BackupFixture {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("operit-workspace-backup-{}", uuid::Uuid::new_v4()));
            let runtime = root.join("runtime");
            let workspaces = root.join("workspaces");
            std::fs::create_dir_all(&runtime).unwrap();
            std::fs::create_dir_all(&workspaces).unwrap();
            let context = HostManager {
                fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
                runtimeStorageHost: Some(Arc::new(NativeRuntimeStorageHost::new(
                    runtime, workspaces,
                ))),
                ..HostManager::default()
            };
            Self {
                root,
                manager: WorkspaceBackupManager::new(context),
            }
        }

        fn edit(&self, workspace: &str, chat: &str, path: &str, timestamp: i64, content: &str) {
            let session = self.manager.createWorkspaceToolHookSession(
                workspace.to_string(),
                timestamp,
                Some(chat.to_string()),
            );
            session.onToolExecutionStarted(&AITool {
                name: "edit_file".to_string(),
                parameters: vec![ToolParameter {
                    name: "path".to_string(),
                    value: path.to_string(),
                }],
            });
            self.manager.vfs().writeFile(path, content, false).unwrap();
            session.close();
        }
    }

    impl Drop for BackupFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn snapshots_live_in_operit_and_restore_without_capturing_workspace_metadata() {
        let fixture = BackupFixture::new();
        let manager = &fixture.manager;
        let vfs = manager.vfs();
        let workspace = "/app/workspaces/project";
        vfs.makeDirectory(&joinPath(workspace, ".operit"), true)
            .unwrap();
        let file = joinPath(workspace, "main.rs");
        vfs.writeFile(&file, "before\n", false).unwrap();
        vfs.writeFile(&joinPath(workspace, ".operit/config.json"), "{}", false)
            .unwrap();
        let session =
            manager.createWorkspaceToolHookSession(workspace.into(), 100, Some("chat-a".into()));
        // Capturing a whole directory must exclude its configuration and backup objects.
        session.onToolExecutionStarted(&AITool {
            name: "delete_file".into(),
            parameters: vec![ToolParameter {
                name: "path".into(),
                value: workspace.into(),
            }],
        });
        vfs.writeFile(&file, "after\n", false).unwrap();
        session.close();

        let backup = fixture
            .root
            .join("workspaces/project/.operit/backups/chat-a");
        assert!(backup.join("state.json").is_file());
        assert!(backup.join("objects").is_dir());
        assert!(!fixture.root.join("runtime/workspace_backups").exists());
        let snapshot = manager
            .loadTurnSnapshot(&vfs, workspace, Some("chat-a"))
            .unwrap();
        assert_eq!(
            snapshot
                .files
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["main.rs"]
        );
        assert_eq!(
            manager
                .previewChangesForMessage(workspace.into(), 100, Some("chat-a".into()))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            manager.restoreWorkspaceForMessage(workspace.into(), 100, Some("chat-a".into())),
            WorkspaceRewindOutcome::Restored
        );
        assert_eq!(vfs.readFile(&file).unwrap(), "before\n");
        assert_eq!(
            vfs.readFile(&joinPath(workspace, ".operit/config.json"))
                .unwrap(),
            "{}"
        );
    }

    #[test]
    fn deleting_chat_backups_is_scoped_to_its_workspace_and_preserves_other_operit_data() {
        let fixture = BackupFixture::new();
        let manager = &fixture.manager;
        let vfs = manager.vfs();
        for workspace in ["/app/workspaces/first", "/app/workspaces/second"] {
            vfs.makeDirectory(&joinPath(workspace, ".operit"), true)
                .unwrap();
            vfs.writeFile(&joinPath(workspace, ".operit/config.json"), "{}", false)
                .unwrap();
            let file = joinPath(workspace, "main.rs");
            vfs.writeFile(&file, "original", false).unwrap();
            fixture.edit(workspace, "chat-a", &file, 100, "edited");
        }
        let first = "/app/workspaces/first";
        let other = joinPath(first, "other.rs");
        vfs.writeFile(&other, "original", false).unwrap();
        fixture.edit(first, "chat-b", &other, 200, "edited");

        manager.deleteChatState(first.into(), Some("chat-a".into()));

        assert!(
            !vfs.fileExists(&manager.chatBackupDir(first, Some("chat-a")))
                .unwrap()
                .exists
        );
        assert!(manager
            .loadTurnSnapshot(&vfs, "/app/workspaces/second", Some("chat-a"))
            .is_some());
        assert!(manager
            .loadTurnSnapshot(&vfs, first, Some("chat-b"))
            .is_some());
        assert_eq!(
            vfs.readFile(&joinPath(first, ".operit/config.json"))
                .unwrap(),
            "{}"
        );
    }

    fn present(hash: &str) -> Capture {
        Capture::Present(hash.to_string())
    }

    #[test]
    fn merge_turn_files_keeps_only_real_changes() {
        let before = BTreeMap::from([
            ("same.rs".to_string(), present("same")),
            ("modified.rs".to_string(), present("old")),
            ("deleted.rs".to_string(), present("gone")),
        ]);
        let after = BTreeMap::from([
            ("same.rs".to_string(), present("same")),
            ("modified.rs".to_string(), present("new")),
            ("added.rs".to_string(), present("added")),
        ]);

        let files = mergeTurnFiles(&before, &after);

        assert_eq!(files.len(), 3);
        assert!(!files.contains_key("same.rs"));
        let modified = files.get("modified.rs").expect("modified entry");
        assert_eq!(modified.before.as_deref(), Some("old"));
        assert_eq!(modified.after.as_deref(), Some("new"));
        let deleted = files.get("deleted.rs").expect("deleted entry");
        assert_eq!(deleted.before.as_deref(), Some("gone"));
        assert_eq!(deleted.after, None);
        let added = files.get("added.rs").expect("added entry");
        assert_eq!(added.before, None);
        assert_eq!(added.after.as_deref(), Some("added"));
    }

    #[test]
    fn merge_turn_files_never_restores_unreadable_captures() {
        let before = BTreeMap::from([("a.rs".to_string(), Capture::Unknown)]);
        let after = BTreeMap::from([("a.rs".to_string(), present("new"))]);

        assert!(mergeTurnFiles(&before, &after).is_empty());
    }

    #[test]
    fn affected_paths_cover_their_descendants() {
        let affected = BTreeSet::from(["/app/workspaces/w/dir".to_string()]);

        assert!(isCoveredByAffectedPath(&affected, "/app/workspaces/w/dir"));
        assert!(isCoveredByAffectedPath(
            &affected,
            "/app/workspaces/w/dir/new.rs"
        ));
        assert!(!isCoveredByAffectedPath(
            &affected,
            "/app/workspaces/w/other.rs"
        ));
    }

    #[test]
    fn rewind_availability_uses_the_latest_editing_turn_as_watermark() {
        assert_eq!(
            rewindAvailability(200, 100),
            RewindAvailability::Unavailable
        );
        assert_eq!(
            rewindAvailability(200, 200),
            RewindAvailability::UseSnapshot
        );
        assert_eq!(
            rewindAvailability(200, 300),
            RewindAvailability::NoRestoreNeeded
        );
        assert_eq!(
            rewindAvailability(0, 100),
            RewindAvailability::NoRestoreNeeded
        );
    }
}
