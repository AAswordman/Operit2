use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::{FileSystemHost, FindFilesRequest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::AITool;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::AIToolHook::AIToolHook;
use crate::ui::features::chat::webview::workspace::process::GitIgnoreFilter::GitIgnoreFilter;

const BACKUP_DIR_NAME: &str = ".backup";
const OBJECTS_DIR_NAME: &str = "objects";
const CHAT_BACKUPS_DIR_NAME: &str = "chats";
const CURRENT_STATE_FILE_NAME: &str = "current_state.json";

const WORKSPACE_MUTATING_TOOLS: [&str; 8] = [
    "apply_file",
    "create_file",
    "edit_file",
    "write_file",
    "write_file_binary",
    "move_file",
    "delete_file",
    "copy_file",
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
    workspaceEnv: Option<String>,
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
        workspaceEnv: Option<String>,
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
            workspaceEnv,
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
    pub fn syncState(
        &self,
        workspacePath: String,
        messageTimestamp: i64,
        workspaceEnv: Option<String>,
        chatId: Option<String>,
    ) {
        self.syncStateProvider(&workspacePath, workspaceEnv.as_deref(), messageTimestamp, chatId.as_deref());
    }

    #[allow(non_snake_case)]
    pub fn previewChanges(
        &self,
        workspacePath: String,
        targetTimestamp: i64,
        workspaceEnv: Option<String>,
        chatId: Option<String>,
    ) -> Vec<WorkspaceFileChange> {
        self.previewChangesProvider(
            &workspacePath,
            workspaceEnv.as_deref(),
            targetTimestamp,
            chatId.as_deref(),
        )
    }

    #[allow(non_snake_case)]
    pub fn previewChangesForRewind(
        &self,
        workspacePath: String,
        workspaceEnv: Option<String>,
        rewindTimestamp: i64,
        chatId: Option<String>,
    ) -> Vec<WorkspaceFileChange> {
        let Some(host) = self.host() else {
            return Vec::new();
        };
        let backupRootDir = joinPath(&workspacePath, BACKUP_DIR_NAME);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId.as_deref());
        let existingBackups = listBackupsInBackupDir(host.as_ref(), &backupDir);
        let newerBackups = existingBackups
            .into_iter()
            .filter(|timestamp| *timestamp > rewindTimestamp)
            .collect::<Vec<_>>();
        let Some(restoreTimestamp) = newerBackups.first().copied() else {
            return Vec::new();
        };
        self.previewChangesProvider(
            &workspacePath,
            workspaceEnv.as_deref(),
            restoreTimestamp,
            chatId.as_deref(),
        )
    }

    fn host(&self) -> Option<Arc<dyn FileSystemHost>> {
        self.context.fileSystemHost.clone()
    }

    fn initializeHookSessionProvider(
        &self,
        workspacePath: &str,
        workspaceEnv: Option<&str>,
        messageTimestamp: i64,
        chatId: Option<&str>,
    ) -> Option<HookSessionInit> {
        let host = self.host()?;
        let workspaceInfo = host.fileExists(workspacePath).ok()?;
        if !workspaceInfo.exists || !workspaceInfo.isDirectory {
            return None;
        }

        let backupRootDir = joinPath(workspacePath, BACKUP_DIR_NAME);
        ensureDirectory(host.as_ref(), &backupRootDir);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId);
        ensureDirectory(host.as_ref(), &backupDir);
        let objectsDir = joinPath(&backupRootDir, OBJECTS_DIR_NAME);
        ensureDirectory(host.as_ref(), &objectsDir);

        let existingBackups = listBackupsInBackupDir(host.as_ref(), &backupDir);
        let targetManifestPath = joinPath(&backupDir, &format!("{messageTimestamp}.json"));
        let hasTargetManifest = host
            .fileExists(&targetManifestPath)
            .map(|value| value.exists)
            .unwrap_or(false);
        let gitignoreRules = self.loadGitignoreRulesProvider(workspacePath, workspaceEnv);

        let mut currentState = self.loadCurrentStateManifestProvider(&backupDir);
        if currentState.is_none() && hasTargetManifest {
            currentState = self.loadBackupManifestProvider(&backupDir, messageTimestamp);
        }
        if currentState.is_none() {
            if let Some(latestTimestamp) = existingBackups.last().copied() {
                currentState = self.loadBackupManifestProvider(&backupDir, latestTimestamp);
            }
        }
        let currentState = currentState.unwrap_or_else(|| BackupManifest {
            timestamp: messageTimestamp,
            files: BTreeMap::new(),
            fileStats: BTreeMap::new(),
        });

        if !hasTargetManifest {
            self.writeBackupManifestProvider(
                &backupDir,
                messageTimestamp,
                &BackupManifest {
                    timestamp: messageTimestamp,
                    ..currentState.clone()
                },
            );
        }
        self.saveCurrentStateManifestProvider(&backupDir, &currentState);

        Some(HookSessionInit {
            backupDir,
            objectsDir,
            currentState,
            gitignoreRules,
        })
    }

    fn loadCurrentStateManifestProvider(&self, backupDir: &str) -> Option<BackupManifest> {
        let statePath = joinPath(backupDir, CURRENT_STATE_FILE_NAME);
        let content = self.host()?.readFile(&statePath).ok()?;
        if content.trim().is_empty() {
            return None;
        }
        serde_json::from_str(&content).ok()
    }

    fn saveCurrentStateManifestProvider(&self, backupDir: &str, manifest: &BackupManifest) {
        let Some(host) = self.host() else {
            return;
        };
        let statePath = joinPath(backupDir, CURRENT_STATE_FILE_NAME);
        let content = serde_json::to_string(manifest).expect("BackupManifest must serialize");
        let _ = host.writeFile(&statePath, &content, false);
    }

    fn writeBackupManifestProvider(
        &self,
        backupDir: &str,
        timestamp: i64,
        manifest: &BackupManifest,
    ) {
        let Some(host) = self.host() else {
            return;
        };
        let manifestPath = joinPath(backupDir, &format!("{timestamp}.json"));
        let content = serde_json::to_string(manifest).expect("BackupManifest must serialize");
        let _ = host.writeFile(&manifestPath, &content, false);
    }

    fn refreshPathInStateProvider(
        &self,
        workspacePath: &str,
        targetPath: &str,
        objectsDir: &str,
        gitignoreRules: &[String],
        files: &mut BTreeMap<String, String>,
        stats: &mut BTreeMap<String, FileStat>,
    ) {
        let Some(host) = self.host() else {
            return;
        };
        let normalizedTargetPath = targetPath.trim().trim_end_matches('/');
        let relativeTarget = match makeRelativePath(workspacePath, normalizedTargetPath) {
            Some(value) => value,
            None => return,
        };

        removePathFromState(&relativeTarget, files, stats);

        let Ok(existsData) = host.fileExists(normalizedTargetPath) else {
            return;
        };
        if !existsData.exists {
            return;
        }

        if existsData.isDirectory {
            let childFiles = self.listWorkspaceTextFilesUnderPathProvider(
                workspacePath,
                normalizedTargetPath,
                gitignoreRules,
            );
            for childPath in childFiles {
                let Some(relativeChildPath) = makeRelativePath(workspacePath, &childPath) else {
                    continue;
                };
                let Some((hash, stat)) = self.snapshotFileForStateProvider(&childPath, objectsDir) else {
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

        if let Some((hash, stat)) = self.snapshotFileForStateProvider(normalizedTargetPath, objectsDir) {
            files.insert(relativeTarget.clone(), hash);
            stats.insert(relativeTarget, stat);
        }
    }

    fn listWorkspaceTextFilesUnderPathProvider(
        &self,
        workspacePath: &str,
        startPath: &str,
        gitignoreRules: &[String],
    ) -> Vec<String> {
        let Some(host) = self.host() else {
            return Vec::new();
        };
        let Ok(allFiles) = host.findFiles(FindFilesRequest {
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

    fn snapshotFileForStateProvider(&self, filePath: &str, objectsDir: &str) -> Option<(String, FileStat)> {
        let host = self.host()?;
        let bytes = host.readFileBytes(filePath).ok()?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let info = host.fileInfo(filePath).ok();
        let stat = FileStat {
            size: info.as_ref().map(|value| value.size).unwrap_or(bytes.len() as i64),
            lastModified: info
                .as_ref()
                .and_then(|value| parseLastModifiedToMillis(&value.lastModified))
                .unwrap_or(0),
        };

        let objectPath = buildShardedObjectPath(objectsDir, &hash);
        let objectExists = host
            .fileExists(&objectPath)
            .map(|value| value.exists)
            .unwrap_or(false);
        if !objectExists {
            let bucketDir = joinPath(objectsDir, &objectBucketPrefix(&hash));
            ensureDirectory(host.as_ref(), &bucketDir);
            let _ = host.writeFileBytes(&objectPath, &bytes);
        }
        Some((hash, stat))
    }

    fn loadBackupManifestProvider(&self, backupDir: &str, targetTimestamp: i64) -> Option<BackupManifest> {
        let manifestPath = joinPath(backupDir, &format!("{targetTimestamp}.json"));
        let content = self.host()?.readFile(&manifestPath).ok()?;
        if content.trim().is_empty() {
            return None;
        }
        serde_json::from_str(&content).ok()
    }

    fn loadGitignoreRulesProvider(&self, workspacePath: &str, _workspaceEnv: Option<&str>) -> Vec<String> {
        let Some(host) = self.host() else {
            return GitIgnoreFilter::defaultRules();
        };
        let gitignorePath = joinPath(workspacePath, ".gitignore");
        match host.readFile(&gitignorePath) {
            Ok(content) if !content.trim().is_empty() => GitIgnoreFilter::buildRulesFromContent(&content),
            _ => GitIgnoreFilter::defaultRules(),
        }
    }

    fn syncStateProvider(
        &self,
        workspacePath: &str,
        workspaceEnv: Option<&str>,
        messageTimestamp: i64,
        chatId: Option<&str>,
    ) {
        let Some(host) = self.host() else {
            return;
        };
        let Ok(exists) = host.fileExists(workspacePath) else {
            return;
        };
        if !exists.exists || !exists.isDirectory {
            return;
        }

        let backupRootDir = joinPath(workspacePath, BACKUP_DIR_NAME);
        ensureDirectory(host.as_ref(), &backupRootDir);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId);
        ensureDirectory(host.as_ref(), &backupDir);
        let objectsDir = joinPath(&backupRootDir, OBJECTS_DIR_NAME);
        ensureDirectory(host.as_ref(), &objectsDir);

        let existingBackups = listBackupsInBackupDir(host.as_ref(), &backupDir);
        let mut currentState = self.loadCurrentStateManifestProvider(&backupDir);
        if currentState.is_none() {
            if let Some(latestTimestamp) = existingBackups.last().copied() {
                currentState = self.loadBackupManifestProvider(&backupDir, latestTimestamp);
            }
            let currentStateValue = currentState.clone().unwrap_or_else(|| BackupManifest {
                timestamp: messageTimestamp,
                files: BTreeMap::new(),
                fileStats: BTreeMap::new(),
            });
            self.saveCurrentStateManifestProvider(&backupDir, &currentStateValue);
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
            let targetManifest = self.loadBackupManifestProvider(&backupDir, restoreTimestamp);
            self.restoreFromManifestsProvider(
                workspacePath,
                workspaceEnv,
                &objectsDir,
                &currentStateValue,
                targetManifest.as_ref(),
            );
            let restoredState = targetManifest.unwrap_or_else(|| BackupManifest {
                timestamp: restoreTimestamp,
                files: BTreeMap::new(),
                fileStats: BTreeMap::new(),
            });
            self.saveCurrentStateManifestProvider(&backupDir, &restoredState);
            for timestamp in newerBackups {
                let _ = host.deleteFile(&joinPath(&backupDir, &format!("{timestamp}.json")), false);
            }
            return;
        }

        if existingBackups.contains(&messageTimestamp) {
            if let Some(existingManifest) = self.loadBackupManifestProvider(&backupDir, messageTimestamp) {
                self.saveCurrentStateManifestProvider(&backupDir, &existingManifest);
            }
            return;
        }

        self.writeBackupManifestProvider(
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
        workspacePath: &str,
        _workspaceEnv: Option<&str>,
        objectsDir: &str,
        currentState: &BackupManifest,
        targetManifest: Option<&BackupManifest>,
    ) {
        let Some(host) = self.host() else {
            return;
        };
        let targetFiles = targetManifest
            .map(|manifest| manifest.files.clone())
            .unwrap_or_default();
        for relativePath in currentState.files.keys() {
            if targetFiles.contains_key(relativePath) {
                continue;
            }
            let currentFilePath = joinPath(workspacePath, relativePath);
            let _ = host.deleteFile(&currentFilePath, false);
        }

        for (relativePath, hash) in targetFiles {
            if currentState.files.get(&relativePath) == Some(&hash) {
                continue;
            }
            let Some(objectPath) = resolveObjectPathForRead(host.as_ref(), objectsDir, &hash) else {
                continue;
            };
            let Ok(bytes) = host.readFileBytes(&objectPath) else {
                continue;
            };
            let targetPath = joinPath(workspacePath, &relativePath);
            let parent = targetPath.rsplit_once('/').map(|(parent, _)| parent).unwrap_or("");
            if !parent.is_empty() {
                ensureDirectory(host.as_ref(), parent);
            }
            let _ = host.writeFileBytes(&targetPath, &bytes);
        }
    }

    fn loadCurrentStateForDiffProvider(&self, backupDir: &str) -> BackupManifest {
        if let Some(currentState) = self.loadCurrentStateManifestProvider(backupDir) {
            return currentState;
        }

        let Some(host) = self.host() else {
            return BackupManifest {
                timestamp: currentTimeMillis(),
                files: BTreeMap::new(),
                fileStats: BTreeMap::new(),
            };
        };
        if let Some(latestTimestamp) = listBackupsInBackupDir(host.as_ref(), backupDir).last().copied() {
            if let Some(latestManifest) = self.loadBackupManifestProvider(backupDir, latestTimestamp) {
                return latestManifest;
            }
        }

        BackupManifest {
            timestamp: currentTimeMillis(),
            files: BTreeMap::new(),
            fileStats: BTreeMap::new(),
        }
    }

    fn readTextFromObjectHashProvider(&self, objectsDir: &str, hash: &str) -> Option<String> {
        let host = self.host()?;
        let objectPath = resolveObjectPathForRead(host.as_ref(), objectsDir, hash)?;
        let bytes = host.readFileBytes(&objectPath).ok()?;
        String::from_utf8(bytes).ok()
    }

    fn estimateLineCountFromHashProvider(&self, objectsDir: &str, hash: &str) -> i32 {
        let Some(text) = self.readTextFromObjectHashProvider(objectsDir, hash) else {
            return 0;
        };
        normalizeTextLinesForDiff(&text).len() as i32
    }

    fn estimateChangedLinesBetweenHashesProvider(
        &self,
        objectsDir: &str,
        currentHash: &str,
        targetHash: &str,
    ) -> i32 {
        if currentHash == targetHash {
            return 0;
        }
        let Some(currentText) = self.readTextFromObjectHashProvider(objectsDir, currentHash) else {
            return 0;
        };
        let Some(targetText) = self.readTextFromObjectHashProvider(objectsDir, targetHash) else {
            return 0;
        };
        estimateChangedLines(&currentText, &targetText)
    }

    fn previewChangesProvider(
        &self,
        workspacePath: &str,
        _workspaceEnv: Option<&str>,
        targetTimestamp: i64,
        chatId: Option<&str>,
    ) -> Vec<WorkspaceFileChange> {
        let Some(host) = self.host() else {
            return Vec::new();
        };
        let Ok(exists) = host.fileExists(workspacePath) else {
            return Vec::new();
        };
        if !exists.exists || !exists.isDirectory {
            return Vec::new();
        }

        let backupRootDir = joinPath(workspacePath, BACKUP_DIR_NAME);
        let backupDir = resolveChatBackupDir(&backupRootDir, chatId);
        let objectsDir = joinPath(&backupRootDir, OBJECTS_DIR_NAME);
        let currentState = self.loadCurrentStateForDiffProvider(&backupDir);
        let targetManifest = self
            .loadBackupManifestProvider(&backupDir, targetTimestamp)
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
                let deletedLines = self.estimateLineCountFromHashProvider(&objectsDir, currentHash);
                changes.push(WorkspaceFileChange {
                    path: relativePath.clone(),
                    changeType: ChangeType::DELETED,
                    changedLines: deletedLines,
                });
                continue;
            };

            if targetHash != currentHash {
                let changedLines = self.estimateChangedLinesBetweenHashesProvider(
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
            let addedLines = self.estimateLineCountFromHashProvider(&objectsDir, targetHash);
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
        let state = self.state.lock().expect("WorkspaceToolHookSession mutex poisoned");
        if !state.initialized {
            return;
        }
        if let (Some(backupDir), Some(currentState)) =
            (state.backupDir.as_deref(), state.currentState.as_ref())
        {
            self.manager.saveCurrentStateManifestProvider(backupDir, currentState);
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
        let affectedPaths = extractWorkspaceAffectedPaths(tool, &self.workspacePath, self.workspaceEnv.as_deref());
        if affectedPaths.is_empty() {
            return;
        }

        let mut state = self.state.lock().expect("WorkspaceToolHookSession mutex poisoned");
        if state.initialized {
            return;
        }
        let Some(init) = self.manager.initializeHookSessionProvider(
            &self.workspacePath,
            self.workspaceEnv.as_deref(),
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
        if self.closed.load(Ordering::SeqCst) || !result.success || !isWorkspaceMutatingTool(&tool.name) {
            return;
        }
        let affectedPaths = extractWorkspaceAffectedPaths(tool, &self.workspacePath, self.workspaceEnv.as_deref());
        if affectedPaths.is_empty() {
            return;
        }

        let mut state = self.state.lock().expect("WorkspaceToolHookSession mutex poisoned");
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
        for affectedPath in distinctPaths {
            self.manager.refreshPathInStateProvider(
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

fn isEnvironmentMatchForWorkspace(toolEnv: Option<&str>, workspaceEnv: Option<&str>) -> bool {
    let normalizedToolEnv = toolEnv.unwrap_or("").trim();
    let normalizedWorkspaceEnv = workspaceEnv.unwrap_or("").trim();
    if normalizedWorkspaceEnv.is_empty() {
        return normalizedToolEnv.is_empty() || normalizedToolEnv.eq_ignore_ascii_case("android");
    }
    normalizedToolEnv.eq_ignore_ascii_case(normalizedWorkspaceEnv)
}

fn extractWorkspaceAffectedPaths(
    tool: &AITool,
    workspacePath: &str,
    workspaceEnv: Option<&str>,
) -> Vec<String> {
    let mut result = Vec::<String>::new();
    let defaultEnvironment = toolParam(tool, "environment");
    match tool.name.as_str() {
        "apply_file" | "create_file" | "edit_file" | "delete_file" | "write_file" | "write_file_binary" => {
            collectWorkspacePath(&mut result, toolParam(tool, "path"), defaultEnvironment, workspacePath, workspaceEnv);
        }
        "move_file" => {
            collectWorkspacePath(&mut result, toolParam(tool, "source"), defaultEnvironment, workspacePath, workspaceEnv);
            collectWorkspacePath(&mut result, toolParam(tool, "destination"), defaultEnvironment, workspacePath, workspaceEnv);
        }
        "copy_file" => {
            let sourceEnvironment = toolParam(tool, "source_environment").or(defaultEnvironment);
            let destinationEnvironment = toolParam(tool, "dest_environment").or(defaultEnvironment);
            collectWorkspacePath(&mut result, toolParam(tool, "source"), sourceEnvironment, workspacePath, workspaceEnv);
            collectWorkspacePath(&mut result, toolParam(tool, "destination"), destinationEnvironment, workspacePath, workspaceEnv);
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

fn collectWorkspacePath(
    result: &mut Vec<String>,
    path: Option<&str>,
    toolEnv: Option<&str>,
    workspacePath: &str,
    workspaceEnv: Option<&str>,
) {
    let Some(rawPath) = path else {
        return;
    };
    let mut normalizedPath = rawPath.trim().trim_end_matches('/').to_string();
    if normalizedPath.is_empty() || !isEnvironmentMatchForWorkspace(toolEnv, workspaceEnv) {
        return;
    }
    if makeRelativePath(workspacePath, &normalizedPath).is_none() && !startsWithAbsoluteRoot(&normalizedPath) {
        normalizedPath = joinPath(workspacePath, &normalizedPath);
    }
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

fn listBackupsInBackupDir(host: &dyn FileSystemHost, backupDir: &str) -> Vec<i64> {
    let Ok(entries) = host.listFiles(backupDir) else {
        return Vec::new();
    };
    let mut timestamps = entries
        .into_iter()
        .filter(|entry| !entry.isDirectory)
        .filter_map(|entry| entry.name.strip_suffix(".json").and_then(|value| value.parse::<i64>().ok()))
        .collect::<Vec<_>>();
    timestamps.sort_unstable();
    timestamps
}

fn ensureDirectory(host: &dyn FileSystemHost, path: &str) {
    let _ = host.makeDirectory(path, true);
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
    joinPath(&joinPath(backupRootDir, CHAT_BACKUPS_DIR_NAME), &normalizeChatScope(chatId))
}

fn joinPath(parent: &str, child: &str) -> String {
    let parent = GitIgnoreFilter::normalizePath(parent);
    let child = GitIgnoreFilter::normalizePath(child).trim_start_matches('/').to_string();
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
    if normalizedFullPath == normalizedRoot {
        return Some(String::new());
    }
    let prefix = format!("{normalizedRoot}/");
    if !normalizedFullPath.starts_with(&prefix) {
        return None;
    }
    Some(normalizedFullPath[prefix.len()..].trim_start_matches('/').to_string())
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

fn resolveObjectPathForRead(host: &dyn FileSystemHost, objectsDir: &str, hash: &str) -> Option<String> {
    let sharded = buildShardedObjectPath(objectsDir, hash);
    if host.fileExists(&sharded).map(|value| value.exists).unwrap_or(false) {
        return Some(sharded);
    }
    let legacy = buildLegacyObjectPath(objectsDir, hash);
    if host.fileExists(&legacy).map(|value| value.exists).unwrap_or(false) {
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
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis() as i64
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
    let extension = lower.rsplit_once('.').map(|(_, extension)| extension).unwrap_or("");
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
