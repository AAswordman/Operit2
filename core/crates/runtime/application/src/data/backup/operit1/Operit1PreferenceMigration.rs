#[derive(Clone, Debug)]
struct SnapshotFileImportPlan {
    importedFilesRoot: String,
    importedExternalFilesRoot: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Operit1SnapshotFileImportResult {
    importedFiles: i32,
    importedExternalFiles: i32,
    importedWorkspaces: i32,
    importedWorkspaceFiles: i32,
}

#[derive(Clone, Debug)]
/// Describes one archive entry and its final runtime storage destination.
struct SnapshotFileCopyItem {
    sourceEntry: String,
    targetPath: String,
}

#[derive(Clone, Debug)]
/// Collects every resource file that must be copied from one Operit1 snapshot.
struct SnapshotFileCopyPlan {
    items: Vec<SnapshotFileCopyItem>,
    workspaceIds: BTreeSet<String>,
    importedFiles: i32,
    importedExternalFiles: i32,
    importedWorkspaceFiles: i32,
}

impl SnapshotFileImportPlan {
    /// Creates a file import plan using stable virtual file-system destinations.
    fn new() -> Self {
        Self {
            importedFilesRoot: RUNTIME_IMPORTED_OPERIT1_FILES_DIR_PATH.to_string(),
            importedExternalFilesRoot: RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES_DIR_PATH.to_string(),
        }
    }

    #[allow(non_snake_case)]
    fn rewritePreferenceValue(&self, key: &str, value: &str) -> Result<String, String> {
        if isOperit1WorkspaceStatePreferenceKey(key) {
            self.rewriteWorkspaceStatePreferenceValue(key, value)
        } else if isOperit1PathPreferenceKey(key) {
            self.rewritePath(value)
        } else {
            Ok(value.to_string())
        }
    }

    #[allow(non_snake_case)]
    fn rewritePreferenceKey(&self, key: &str) -> Result<String, String> {
        if let Some((prefix, workspace)) = splitOperit1WorkspaceStatePreferenceKey(key) {
            let targetWorkspace = self.rewriteWorkspacePath(workspace)?;
            return Ok(format!("{prefix}{targetWorkspace}"));
        }
        Ok(key.to_string())
    }

    #[allow(non_snake_case)]
    fn rewriteWorkspaceStatePreferenceValue(
        &self,
        key: &str,
        value: &str,
    ) -> Result<String, String> {
        if splitOperit1WorkspaceStatePreferenceKey(key)
            .map(|(prefix, _)| prefix)
            .map(isOperit1WorkspaceStateScalarPreferencePrefix)
            == Some(true)
        {
            return Ok(value.to_string());
        }
        let mut json: Value = serde_json::from_str(value)
            .map_err(|error| format!("Operit1 工作区状态不是合法 JSON：{error}"))?;
        let Some(items) = json.as_array_mut() else {
            return Err("Operit1 工作区状态不是数组".to_string());
        };
        for item in items {
            let Some(path) = item
                .as_object_mut()
                .and_then(|object| object.get_mut("path"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
            else {
                continue;
            };
            let rewritten = self.rewriteWorkspacePath(&path)?;
            if let Some(object) = item.as_object_mut() {
                object.insert("path".to_string(), Value::String(rewritten));
            }
        }
        serde_json::to_string(&json).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn rewritePath(&self, value: &str) -> Result<String, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(value.to_string());
        }
        let pathText = trimmed.replace('\\', "/");
        let localPath = pathText.strip_prefix("file://").unwrap_or(&pathText);
        if let Some(relative) = localPath.strip_prefix(OPERIT1_INTERNAL_FILES_PREFIX) {
            return self.rewriteInternalFilesRelativePath(relative);
        }
        if let Some(relative) = localPath.strip_prefix(OPERIT1_DATA_DATA_FILES_PREFIX) {
            return self.rewriteInternalFilesRelativePath(relative);
        }
        if let Some(relative) = localPath.strip_prefix(OPERIT1_EXTERNAL_DOWNLOAD_PREFIX) {
            return self.rewriteExternalDownloadRelativePath(relative);
        }
        Ok(value.to_string())
    }

    #[allow(non_snake_case)]
    /// Rewrites the persisted chat workspace binding into the current virtual path space.
    fn rewriteChatWorkspace(&self, workspace: Option<String>) -> Result<Option<String>, String> {
        let Some(workspace) = workspace else {
            return Ok(None);
        };
        self.rewritePath(&workspace).map(Some)
    }

    #[allow(non_snake_case)]
    fn rewriteWorkspacePath(&self, workspace: &str) -> Result<String, String> {
        let pathText = workspace.trim().replace('\\', "/");
        if pathText.is_empty() {
            return Ok(workspace.to_string());
        }
        if let Some(relative) = pathText.strip_prefix(OPERIT1_INTERNAL_FILES_PREFIX) {
            return self.rewriteWorkspaceRelativePath(relative);
        }
        if let Some(relative) = pathText.strip_prefix(OPERIT1_DATA_DATA_FILES_PREFIX) {
            return self.rewriteWorkspaceRelativePath(relative);
        }
        if let Some(relative) = pathText.strip_prefix(OPERIT1_EXTERNAL_DOWNLOAD_PREFIX) {
            return self.rewriteWorkspaceRelativePath(relative);
        }
        Ok(workspace.to_string())
    }

    #[allow(non_snake_case)]
    fn rewriteInternalFilesRelativePath(&self, relative: &str) -> Result<String, String> {
        validateRelativePath(relative)?;
        if let Some(workspaceRelative) = relative.strip_prefix("workspace/") {
            let (workspaceId, rest) = splitWorkspaceRelativePath(workspaceRelative)?;
            return Ok(workspaceVfsPath(workspaceId, rest));
        }
        Ok(joinVirtualPath(&self.importedFilesRoot, relative))
    }

    #[allow(non_snake_case)]
    fn rewriteExternalDownloadRelativePath(&self, relative: &str) -> Result<String, String> {
        validateRelativePath(relative)?;
        if let Some(workspaceRelative) = relative.strip_prefix("workspace/") {
            let (workspaceId, rest) = splitWorkspaceRelativePath(workspaceRelative)?;
            return Ok(workspaceVfsPath(workspaceId, rest));
        }
        Ok(joinVirtualPath(&self.importedExternalFilesRoot, relative))
    }

    #[allow(non_snake_case)]
    fn rewriteWorkspaceRelativePath(&self, relative: &str) -> Result<String, String> {
        validateRelativePath(relative)?;
        let workspaceRelative = relative
            .strip_prefix("workspace/")
            .ok_or_else(|| format!("Operit1 工作区路径不是 workspace 子目录：{relative}"))?;
        let (workspaceId, rest) = splitWorkspaceRelativePath(workspaceRelative)?;
        Ok(workspaceVfsPath(workspaceId, rest))
    }
}

#[allow(non_snake_case)]
fn isOperit1PathPreferenceKey(key: &str) -> bool {
    matches!(
        key,
        "background_image_uri"
            | "bubble_user_image_uri"
            | "bubble_ai_image_uri"
            | "custom_user_avatar_uri"
            | "custom_ai_avatar_uri"
            | "global_user_avatar_uri"
            | "custom_font_path"
            | "bubble_user_custom_font_path"
            | "bubble_ai_custom_font_path"
    ) || key.ends_with("_background_image_uri")
        || key.ends_with("_bubble_user_image_uri")
        || key.ends_with("_bubble_ai_image_uri")
        || key.ends_with("_custom_user_avatar_uri")
        || key.ends_with("_custom_ai_avatar_uri")
        || key.ends_with("_global_user_avatar_uri")
        || key.ends_with("_custom_font_path")
        || key.ends_with("_bubble_user_custom_font_path")
        || key.ends_with("_bubble_ai_custom_font_path")
}

#[allow(non_snake_case)]
fn isOperit1WorkspaceStatePreferenceKey(key: &str) -> bool {
    splitOperit1WorkspaceStatePreferenceKey(key).is_some()
}

#[allow(non_snake_case)]
fn splitOperit1WorkspaceStatePreferenceKey(key: &str) -> Option<(&'static str, &str)> {
    for prefix in [
        "open_files_",
        "unsaved_files_",
        "export_version_code_",
        "export_version_name_",
    ] {
        if let Some(workspace) = key.strip_prefix(prefix) {
            return Some((prefix, workspace));
        }
    }
    None
}

#[allow(non_snake_case)]
fn isOperit1WorkspaceStateScalarPreferencePrefix(prefix: &str) -> bool {
    matches!(prefix, "export_version_code_" | "export_version_name_")
}

#[allow(non_snake_case)]
fn isDataStoreEntry(entry: &str) -> bool {
    entry.starts_with(ENTRY_DATASTORE_PREFIX) && entry.ends_with(".preferences_pb")
}

#[allow(non_snake_case)]
fn datastorePreferenceMappings(paths: &RuntimeStorePaths) -> BTreeMap<String, PathBuf> {
    let mut mappings = BTreeMap::new();
    mappings.insert(
        "payload/files/datastore/current_chat_id.preferences_pb".to_string(),
        paths.current_chat_id_preferences_path(),
    );
    mappings.insert(
        "payload/files/datastore/tool_permissions.preferences_pb".to_string(),
        paths.tool_permissions_preferences_path(),
    );
    mappings.insert(
        "payload/files/datastore/user_preferences.preferences_pb".to_string(),
        paths.runtime_storage_path(USER_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/api_settings.preferences_pb".to_string(),
        paths.runtime_storage_path(API_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/display_preferences.preferences_pb".to_string(),
        paths.runtime_storage_path(DISPLAY_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/ui_preferences.preferences_pb".to_string(),
        paths.runtime_storage_path(UI_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/waifu_settings.preferences_pb".to_string(),
        paths.runtime_storage_path(WAIFU_SETTINGS_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/wake_word_preferences.preferences_pb".to_string(),
        paths.runtime_storage_path(WAKE_WORD_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/custom_emoji_settings.preferences_pb".to_string(),
        paths.runtime_storage_path(CUSTOM_EMOJI_SETTINGS_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/android_permission_preferences.preferences_pb".to_string(),
        paths.runtime_storage_path(ANDROID_PERMISSION_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/database_backup_settings.preferences_pb".to_string(),
        paths.runtime_storage_path(DATABASE_BACKUP_SETTINGS_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/github_auth_preferences.preferences_pb".to_string(),
        paths.runtime_storage_path(GITHUB_AUTH_PREFERENCES_PATH),
    );
    mappings.insert(
        "payload/files/datastore/persona_card_chat_history.preferences_pb".to_string(),
        paths.runtime_storage_path(PERSONA_CARD_CHAT_HISTORY_PREFERENCES_PATH),
    );
    mappings
}

