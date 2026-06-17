use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use operit_host_api::FindFilesRequest;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::AITool;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::files::PathMapper::PathMapper;
use crate::core::files::VisualFileSystem::VisualFileSystem;
use crate::core::tools::AIToolHook::AIToolHook;
use crate::ui::features::chat::webview::workspace::process::GitIgnoreFilter::GitIgnoreFilter;

const BACKUP_DIR_NAME: &str = ".backup";
const OBJECTS_DIR_NAME: &str = "objects";
const CHAT_BACKUPS_DIR_NAME: &str = "chats";
const CURRENT_STATE_FILE_NAME: &str = "current_state.json";

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
pub struct FileStat {
    pub size: i64,
    pub lastModified: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub timestamp: i64,
    pub files: BTreeMap<String, String>,
    pub fileStats: BTreeMap<String, FileStat>,
}

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

#[derive(Clone)]
pub struct WorkspaceBackupManager {
    context: OperitApplicationContext,
}

struct HookSessionInit {
    backupDir: String,
    objectsDir: String,
    currentState: BackupManifest,
    gitignoreRules: Vec<String>,
}

struct WorkspaceToolHookState {
    initialized: bool,
    backupDir: Option<String>,
    objectsDir: Option<String>,
    currentState: Option<BackupManifest>,
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

impl WorkspaceBackupManager {
    pub fn new(context: OperitApplicationContext) -> Self {
        Self { context }
    }

    #[allow(non_snake_case)]
    pub fn getInstance(context: OperitApplicationContext) -> Self {
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
                backupDir: None,
                objectsDir: None,
                currentState: None,
                gitignoreRules: Vec::new(),
            }),
        })
    }

    #[allow(non_snake_case)]
    pub fn syncState(&self, workspacePath: String, messageTimestamp: i64, chatId: Option<String>) {
        self.syncStateProvider(&workspacePath, messageTimestamp, chatId.as_deref());
    }

    #[allow(non_snake_case)]
    pub fn previewChanges(
        &self,
        workspacePath: String,
        targetTimestamp: i64,
        chatId: Option<String>,
    ) -> Vec<WorkspaceFileChange> {
        self.previewChangesProvider(&workspacePath, targetTimestamp, chatId.as_deref())
    }

    #[allow(non_snake_case)]
    pub fn previewChangesForRewind(
        &self,
        workspacePath: String,
        rewindTimestamp: i64,
        chatId: Option<String>,
    ) -> Vec<WorkspaceFileChange> {
        let vfs = self.vfsForWorkspace(&workspacePath);
        let backupRootDir = joinPath(&workspacePath, BACKUP_DIR_NAME);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId.as_deref());
        let existingBackups = listBackupsInBackupDir(&vfs, &backupDir);
        let newerBackups = existingBackups
            .into_iter()
            .filter(|timestamp| *timestamp > rewindTimestamp)
            .collect::<Vec<_>>();
        let Some(restoreTimestamp) = newerBackups.first().copied() else {
            return Vec::new();
        };
        self.previewChangesProvider(&workspacePath, restoreTimestamp, chatId.as_deref())
    }

    #[allow(non_snake_case)]
    fn vfsForWorkspace(&self, workspacePath: &str) -> VisualFileSystem {
        let runtimeStoreRoot = self
            .context
            .runtimeStorageHost
            .as_ref()
            .and_then(|host| host.rootDir())
            .expect("RuntimeStorageHost root must be configured for WorkspaceBackupManager");
        let runtimeStorePaths = RuntimeStorePaths::new(runtimeStoreRoot.clone());
        VisualFileSystem::new(
            self.context
                .fileSystemHost
                .clone()
                .expect("FileSystemHost must be configured for WorkspaceBackupManager"),
            PathMapper::new(
                runtimeStoreRoot,
                self.context.appFilesRoot.clone(),
                runtimeStorePaths.workspace_dir(),
                Some(workspacePath.to_string()),
            ),
        )
    }

    fn initializeHookSessionProvider(
        &self,
        workspacePath: &str,
        messageTimestamp: i64,
        chatId: Option<&str>,
    ) -> Option<HookSessionInit> {
        let vfs = self.vfsForWorkspace(workspacePath);
        let workspaceInfo = vfs.fileExists(workspacePath).ok()?;
        if !workspaceInfo.exists || !workspaceInfo.isDirectory {
            return None;
        }

        let backupRootDir = joinPath(workspacePath, BACKUP_DIR_NAME);
        ensureDirectory(&vfs, &backupRootDir);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId);
        ensureDirectory(&vfs, &backupDir);
        let objectsDir = joinPath(&backupRootDir, OBJECTS_DIR_NAME);
        ensureDirectory(&vfs, &objectsDir);

        let existingBackups = listBackupsInBackupDir(&vfs, &backupDir);
        let targetManifestPath = joinPath(&backupDir, &format!("{messageTimestamp}.json"));
        let hasTargetManifest = vfs
            .fileExists(&targetManifestPath)
            .map(|value| value.exists)
            .unwrap_or(false);
        let gitignoreRules = self.loadGitignoreRulesProvider(&vfs, workspacePath);

        let mut currentState = self.loadCurrentStateManifestProvider(&vfs, &backupDir);
        if currentState.is_none() && hasTargetManifest {
            currentState = self.loadBackupManifestProvider(&vfs, &backupDir, messageTimestamp);
        }
        if currentState.is_none() {
            if let Some(latestTimestamp) = existingBackups.last().copied() {
                currentState = self.loadBackupManifestProvider(&vfs, &backupDir, latestTimestamp);
            }
        }
        let currentState = currentState.unwrap_or_else(|| BackupManifest {
            timestamp: messageTimestamp,
            files: BTreeMap::new(),
            fileStats: BTreeMap::new(),
        });

        if !hasTargetManifest {
            self.writeBackupManifestProvider(
                &vfs,
                &backupDir,
                messageTimestamp,
                &BackupManifest {
                    timestamp: messageTimestamp,
                    ..currentState.clone()
                },
            );
        }
        self.saveCurrentStateManifestProvider(&vfs, &backupDir, &currentState);

        Some(HookSessionInit {
            backupDir,
            objectsDir,
            currentState,
            gitignoreRules,
        })
    }

    fn loadCurrentStateManifestProvider(
        &self,
        vfs: &VisualFileSystem,
        backupDir: &str,
    ) -> Option<BackupManifest> {
        let statePath = joinPath(backupDir, CURRENT_STATE_FILE_NAME);
        let content = vfs.readFile(&statePath).ok()?;
        if content.trim().is_empty() {
            return None;
        }
        serde_json::from_str(&content).ok()
    }

    fn saveCurrentStateManifestProvider(
        &self,
        vfs: &VisualFileSystem,
        backupDir: &str,
        manifest: &BackupManifest,
    ) {
        let statePath = joinPath(backupDir, CURRENT_STATE_FILE_NAME);
        let content = serde_json::to_string(manifest).expect("BackupManifest must serialize");
        let _ = vfs.writeFile(&statePath, &content, false);
    }

    fn writeBackupManifestProvider(
        &self,
        vfs: &VisualFileSystem,
        backupDir: &str,
        timestamp: i64,
        manifest: &BackupManifest,
    ) {
        let manifestPath = joinPath(backupDir, &format!("{timestamp}.json"));
        let content = serde_json::to_string(manifest).expect("BackupManifest must serialize");
        let _ = vfs.writeFile(&manifestPath, &content, false);
    }

    fn refreshPathInStateProvider(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
        targetPath: &str,
        objectsDir: &str,
        gitignoreRules: &[String],
        files: &mut BTreeMap<String, String>,
        stats: &mut BTreeMap<String, FileStat>,
    ) {
        let normalizedTargetPath = targetPath.trim().trim_end_matches('/');
        let relativeTarget = match makeRelativePath(workspacePath, normalizedTargetPath) {
            Some(value) => value,
            None => return,
        };

        removePathFromState(&relativeTarget, files, stats);

        let Ok(existsData) = vfs.fileExists(normalizedTargetPath) else {
            return;
        };
        if !existsData.exists {
            return;
        }

        if existsData.isDirectory {
            let childFiles = self.listWorkspaceTextFilesUnderPathProvider(
                vfs,
                workspacePath,
                normalizedTargetPath,
                gitignoreRules,
            );
            for childPath in childFiles {
                let Some(relativeChildPath) = makeRelativePath(workspacePath, &childPath) else {
                    continue;
                };
                let Some((hash, stat)) =
                    self.snapshotFileForStateProvider(vfs, &childPath, objectsDir)
                else {
                    continue;
                };
                files.insert(relativeChildPath.clone(), hash);
                stats.insert(relativeChildPath, stat);
            }
            return;
        }

        let fileName = relativeTarget.rsplit('/').next().unwrap_or(&relativeTarget);
        if !isTextBasedFileName(fileName) {
            return;
        }
        if GitIgnoreFilter::shouldIgnore(&relativeTarget, fileName, false, gitignoreRules) {
            return;
        }

        if let Some((hash, stat)) =
            self.snapshotFileForStateProvider(vfs, normalizedTargetPath, objectsDir)
        {
            files.insert(relativeTarget.clone(), hash);
            stats.insert(relativeTarget, stat);
        }
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

    fn snapshotFileForStateProvider(
        &self,
        vfs: &VisualFileSystem,
        filePath: &str,
        objectsDir: &str,
    ) -> Option<(String, FileStat)> {
        let bytes = vfs.readFileBytes(filePath).ok()?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let info = vfs.fileInfo(filePath).ok();
        let stat = FileStat {
            size: info
                .as_ref()
                .map(|value| value.size)
                .unwrap_or(bytes.len() as i64),
            lastModified: info
                .as_ref()
                .and_then(|value| parseLastModifiedToMillis(&value.lastModified))
                .unwrap_or(0),
        };

        let objectPath = buildShardedObjectPath(objectsDir, &hash);
        let objectExists = vfs
            .fileExists(&objectPath)
            .map(|value| value.exists)
            .unwrap_or(false);
        if !objectExists {
            let bucketDir = joinPath(objectsDir, &objectBucketPrefix(&hash));
            ensureDirectory(vfs, &bucketDir);
            let _ = vfs.writeFileBytes(&objectPath, &bytes);
        }
        Some((hash, stat))
    }

    fn loadBackupManifestProvider(
        &self,
        vfs: &VisualFileSystem,
        backupDir: &str,
        targetTimestamp: i64,
    ) -> Option<BackupManifest> {
        let manifestPath = joinPath(backupDir, &format!("{targetTimestamp}.json"));
        let content = vfs.readFile(&manifestPath).ok()?;
        if content.trim().is_empty() {
            return None;
        }
        serde_json::from_str(&content).ok()
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

    fn syncStateProvider(&self, workspacePath: &str, messageTimestamp: i64, chatId: Option<&str>) {
        let vfs = self.vfsForWorkspace(workspacePath);
        let Ok(exists) = vfs.fileExists(workspacePath) else {
            return;
        };
        if !exists.exists || !exists.isDirectory {
            return;
        }

        let backupRootDir = joinPath(workspacePath, BACKUP_DIR_NAME);
        ensureDirectory(&vfs, &backupRootDir);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId);
        ensureDirectory(&vfs, &backupDir);
        let objectsDir = joinPath(&backupRootDir, OBJECTS_DIR_NAME);
        ensureDirectory(&vfs, &objectsDir);

        let existingBackups = listBackupsInBackupDir(&vfs, &backupDir);
        let mut currentState = self.loadCurrentStateManifestProvider(&vfs, &backupDir);
        if currentState.is_none() {
            if let Some(latestTimestamp) = existingBackups.last().copied() {
                currentState = self.loadBackupManifestProvider(&vfs, &backupDir, latestTimestamp);
            }
            let currentStateValue = currentState.clone().unwrap_or_else(|| BackupManifest {
                timestamp: messageTimestamp,
                files: BTreeMap::new(),
                fileStats: BTreeMap::new(),
            });
            self.saveCurrentStateManifestProvider(&vfs, &backupDir, &currentStateValue);
            currentState = Some(currentStateValue);
        }

        let Some(currentStateValue) = currentState else {
            return;
        };
        let newerBackups = existingBackups
            .iter()
            .copied()
            .filter(|timestamp| *timestamp > messageTimestamp)
            .collect::<Vec<_>>();
        if let Some(restoreTimestamp) = newerBackups.first().copied() {
            let targetManifest =
                self.loadBackupManifestProvider(&vfs, &backupDir, restoreTimestamp);
            self.restoreFromManifestsProvider(
                &vfs,
                workspacePath,
                &objectsDir,
                &currentStateValue,
                targetManifest.as_ref(),
            );
            let restoredState = targetManifest.unwrap_or_else(|| BackupManifest {
                timestamp: restoreTimestamp,
                files: BTreeMap::new(),
                fileStats: BTreeMap::new(),
            });
            self.saveCurrentStateManifestProvider(&vfs, &backupDir, &restoredState);
            for timestamp in newerBackups {
                let _ = vfs.deleteFile(&joinPath(&backupDir, &format!("{timestamp}.json")), false);
            }
            return;
        }

        if existingBackups.contains(&messageTimestamp) {
            if let Some(existingManifest) =
                self.loadBackupManifestProvider(&vfs, &backupDir, messageTimestamp)
            {
                self.saveCurrentStateManifestProvider(&vfs, &backupDir, &existingManifest);
            }
            return;
        }

        self.writeBackupManifestProvider(
            &vfs,
            &backupDir,
            messageTimestamp,
            &BackupManifest {
                timestamp: messageTimestamp,
                ..currentStateValue
            },
        );
    }

    fn restoreFromManifestsProvider(
        &self,
        vfs: &VisualFileSystem,
        workspacePath: &str,
        objectsDir: &str,
        currentState: &BackupManifest,
        targetManifest: Option<&BackupManifest>,
    ) {
        let targetFiles = targetManifest
            .map(|manifest| manifest.files.clone())
            .unwrap_or_default();
        for relativePath in currentState.files.keys() {
            if targetFiles.contains_key(relativePath) {
                continue;
            }
            let currentFilePath = joinPath(workspacePath, relativePath);
            let _ = vfs.deleteFile(&currentFilePath, false);
        }

        for (relativePath, hash) in targetFiles {
            if currentState.files.get(&relativePath) == Some(&hash) {
                continue;
            }
            let Some(objectPath) = resolveObjectPathForRead(vfs, objectsDir, &hash) else {
                continue;
            };
            let Ok(bytes) = vfs.readFileBytes(&objectPath) else {
                continue;
            };
            let targetPath = joinPath(workspacePath, &relativePath);
            let parent = targetPath
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .unwrap_or("");
            if !parent.is_empty() {
                ensureDirectory(vfs, parent);
            }
            let _ = vfs.writeFileBytes(&targetPath, &bytes);
        }
    }

    fn loadCurrentStateForDiffProvider(
        &self,
        vfs: &VisualFileSystem,
        backupDir: &str,
    ) -> BackupManifest {
        if let Some(currentState) = self.loadCurrentStateManifestProvider(vfs, backupDir) {
            return currentState;
        }

        if let Some(latestTimestamp) = listBackupsInBackupDir(vfs, backupDir).last().copied() {
            if let Some(latestManifest) =
                self.loadBackupManifestProvider(vfs, backupDir, latestTimestamp)
            {
                return latestManifest;
            }
        }

        BackupManifest {
            timestamp: currentTimeMillis(),
            files: BTreeMap::new(),
            fileStats: BTreeMap::new(),
        }
    }

    fn readTextFromObjectHashProvider(
        &self,
        vfs: &VisualFileSystem,
        objectsDir: &str,
        hash: &str,
    ) -> Option<String> {
        let objectPath = resolveObjectPathForRead(vfs, objectsDir, hash)?;
        let bytes = vfs.readFileBytes(&objectPath).ok()?;
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

    fn previewChangesProvider(
        &self,
        workspacePath: &str,
        targetTimestamp: i64,
        chatId: Option<&str>,
    ) -> Vec<WorkspaceFileChange> {
        let vfs = self.vfsForWorkspace(workspacePath);
        let Ok(exists) = vfs.fileExists(workspacePath) else {
            return Vec::new();
        };
        if !exists.exists || !exists.isDirectory {
            return Vec::new();
        }

        let backupRootDir = joinPath(workspacePath, BACKUP_DIR_NAME);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId);
        let objectsDir = joinPath(&backupRootDir, OBJECTS_DIR_NAME);
        let currentState = self.loadCurrentStateForDiffProvider(&vfs, &backupDir);
        let targetManifest = self
            .loadBackupManifestProvider(&vfs, &backupDir, targetTimestamp)
            .unwrap_or_else(|| BackupManifest {
                timestamp: targetTimestamp,
                files: BTreeMap::new(),
                fileStats: BTreeMap::new(),
            });

        let currentFiles = currentState.files;
        let targetFiles = targetManifest.files;
        let mut changes = Vec::<WorkspaceFileChange>::new();

        for (relativePath, currentHash) in &currentFiles {
            let Some(targetHash) = targetFiles.get(relativePath) else {
                let deletedLines =
                    self.estimateLineCountFromHashProvider(&vfs, &objectsDir, currentHash);
                changes.push(WorkspaceFileChange {
                    path: relativePath.clone(),
                    changeType: ChangeType::DELETED,
                    changedLines: deletedLines,
                });
                continue;
            };

            if targetHash != currentHash {
                let changedLines = self.estimateChangedLinesBetweenHashesProvider(
                    &vfs,
                    &objectsDir,
                    currentHash,
                    targetHash,
                );
                if changedLines > 0 {
                    changes.push(WorkspaceFileChange {
                        path: relativePath.clone(),
                        changeType: ChangeType::MODIFIED,
                        changedLines,
                    });
                }
            }
        }

        for (relativePath, targetHash) in &targetFiles {
            if currentFiles.contains_key(relativePath) {
                continue;
            }
            let addedLines = self.estimateLineCountFromHashProvider(&vfs, &objectsDir, targetHash);
            changes.push(WorkspaceFileChange {
                path: relativePath.clone(),
                changeType: ChangeType::ADDED,
                changedLines: addedLines,
            });
        }

        changes.sort_by(|left, right| left.path.cmp(&right.path));
        changes
    }
}

impl WorkspaceToolHookSession {
    #[allow(non_snake_case)]
    pub fn hookId(&self) -> &str {
        &self.id
    }

    pub fn close(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        let state = self
            .state
            .lock()
            .expect("WorkspaceToolHookSession mutex poisoned");
        if !state.initialized {
            return;
        }
        if let (Some(backupDir), Some(currentState)) =
            (state.backupDir.as_deref(), state.currentState.as_ref())
        {
            let vfs = self.manager.vfsForWorkspace(&self.workspacePath);
            self.manager
                .saveCurrentStateManifestProvider(&vfs, backupDir, currentState);
        }
    }
}

impl AIToolHook for WorkspaceToolHookSession {
    fn id(&self) -> &str {
        &self.id
    }

    fn onToolExecutionStarted(&self, tool: &AITool) {
        if self.closed.load(Ordering::SeqCst) || !isWorkspaceMutatingTool(&tool.name) {
            return;
        }
        let affectedPaths = extractWorkspaceAffectedPaths(tool, &self.workspacePath);
        if affectedPaths.is_empty() {
            return;
        }

        let mut state = self
            .state
            .lock()
            .expect("WorkspaceToolHookSession mutex poisoned");
        if state.initialized {
            return;
        }
        let Some(init) = self.manager.initializeHookSessionProvider(
            &self.workspacePath,
            self.messageTimestamp,
            self.chatScopeId.as_deref(),
        ) else {
            return;
        };
        state.backupDir = Some(init.backupDir);
        state.objectsDir = Some(init.objectsDir);
        state.currentState = Some(init.currentState);
        state.gitignoreRules = init.gitignoreRules;
        state.initialized = true;
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

        let mut state = self
            .state
            .lock()
            .expect("WorkspaceToolHookSession mutex poisoned");
        if !state.initialized {
            return;
        }
        let Some(objectsDir) = state.objectsDir.clone() else {
            return;
        };
        let gitignoreRules = state.gitignoreRules.clone();
        let Some(currentState) = state.currentState.as_mut() else {
            return;
        };
        let mut updatedFiles = currentState.files.clone();
        let mut updatedStats = currentState.fileStats.clone();

        let mut distinctPaths = BTreeSet::new();
        for path in affectedPaths {
            distinctPaths.insert(path);
        }
        let vfs = self.manager.vfsForWorkspace(&self.workspacePath);
        for affectedPath in distinctPaths {
            self.manager.refreshPathInStateProvider(
                &vfs,
                &self.workspacePath,
                &affectedPath,
                &objectsDir,
                &gitignoreRules,
                &mut updatedFiles,
                &mut updatedStats,
            );
        }

        *currentState = BackupManifest {
            timestamp: currentTimeMillis(),
            files: updatedFiles,
            fileStats: updatedStats,
        };
    }
}

fn isWorkspaceMutatingTool(toolName: &str) -> bool {
    WORKSPACE_MUTATING_TOOLS.contains(&toolName)
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
    let Some(relativePath) = makeRelativePath(workspacePath, &normalizedPath) else {
        return;
    };
    if relativePath == BACKUP_DIR_NAME || relativePath.starts_with(&format!("{BACKUP_DIR_NAME}/")) {
        return;
    }
    result.push(normalizedPath);
}

fn startsWithAbsoluteRoot(path: &str) -> bool {
    let normalized = GitIgnoreFilter::normalizePath(path);
    normalized.starts_with('/') || normalized.as_bytes().get(1) == Some(&b':')
}

fn listBackupsInBackupDir(vfs: &VisualFileSystem, backupDir: &str) -> Vec<i64> {
    let Ok(entries) = vfs.listFiles(backupDir) else {
        return Vec::new();
    };
    let mut timestamps = entries
        .into_iter()
        .filter(|entry| !entry.isDirectory)
        .filter_map(|entry| {
            entry
                .name
                .strip_suffix(".json")
                .and_then(|value| value.parse::<i64>().ok())
        })
        .collect::<Vec<_>>();
    timestamps.sort_unstable();
    timestamps
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

fn resolveChatBackupDir(backupRootDir: &str, chatId: Option<&str>) -> String {
    joinPath(
        &joinPath(backupRootDir, CHAT_BACKUPS_DIR_NAME),
        &normalizeChatScope(chatId),
    )
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
    let normalizedRoot = GitIgnoreFilter::normalizePath(root)
        .trim_end_matches('/')
        .to_string();
    if normalizedRoot.is_empty() {
        return None;
    }
    let normalizedFullPath = GitIgnoreFilter::normalizePath(fullPath);
    makeRelativePathForNormalizedRoot(&normalizedRoot, &normalizedFullPath)
        .or_else(|| makeRelativePathForNormalizedRoot("/workspace", &normalizedFullPath))
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

fn objectBucketPrefix(hash: &str) -> String {
    if hash.len() < 2 {
        "__".to_string()
    } else {
        hash[..2].to_string()
    }
}

fn buildShardedObjectPath(objectsDir: &str, hash: &str) -> String {
    joinPath(&joinPath(objectsDir, &objectBucketPrefix(hash)), hash)
}

fn buildLegacyObjectPath(objectsDir: &str, hash: &str) -> String {
    joinPath(objectsDir, hash)
}

fn resolveObjectPathForRead(
    vfs: &VisualFileSystem,
    objectsDir: &str,
    hash: &str,
) -> Option<String> {
    let sharded = buildShardedObjectPath(objectsDir, hash);
    if vfs
        .fileExists(&sharded)
        .map(|value| value.exists)
        .unwrap_or(false)
    {
        return Some(sharded);
    }
    let legacy = buildLegacyObjectPath(objectsDir, hash);
    if vfs
        .fileExists(&legacy)
        .map(|value| value.exists)
        .unwrap_or(false)
    {
        return Some(legacy);
    }
    None
}

fn removePathFromState(
    relativePath: &str,
    files: &mut BTreeMap<String, String>,
    stats: &mut BTreeMap<String, FileStat>,
) {
    if relativePath.is_empty() {
        files.clear();
        stats.clear();
        return;
    }
    let prefix = format!("{relativePath}/");
    files.retain(|path, _| path != relativePath && !path.starts_with(&prefix));
    stats.retain(|path, _| path != relativePath && !path.starts_with(&prefix));
}

fn parseLastModifiedToMillis(lastModified: &str) -> Option<i64> {
    let raw = lastModified.trim();
    if raw.is_empty() {
        return None;
    }
    for pattern in ["%Y-%m-%d %H:%M:%S%.3f", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(value) = chrono::NaiveDateTime::parse_from_str(raw, pattern) {
            return Some(value.and_utc().timestamp_millis());
        }
    }
    None
}

fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
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
