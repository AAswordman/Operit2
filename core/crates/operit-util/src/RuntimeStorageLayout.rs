/// Identifies the data ownership class of one registered runtime storage path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeStorageOwnership {
    Space,
    CoreNode,
    Ephemeral,
}

/// Describes how one registered runtime path definition matches concrete files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeStoragePathShape {
    Exact,
    Tree,
    RelativeFile {
        directorySegments: usize,
        fileName: &'static str,
    },
}

/// Declares one runtime storage path together with its owner and concrete shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeStoragePathDefinition {
    pub path: &'static str,
    pub ownership: RuntimeStorageOwnership,
    pub shape: RuntimeStoragePathShape,
}

impl RuntimeStoragePathDefinition {
    /// Creates one exact-file runtime path definition.
    pub const fn exact(path: &'static str, ownership: RuntimeStorageOwnership) -> Self {
        Self {
            path,
            ownership,
            shape: RuntimeStoragePathShape::Exact,
        }
    }

    /// Creates one descendant-tree runtime path definition.
    pub const fn tree(path: &'static str, ownership: RuntimeStorageOwnership) -> Self {
        Self {
            path,
            ownership,
            shape: RuntimeStoragePathShape::Tree,
        }
    }

    /// Creates one relative-file runtime path definition below dynamic directories.
    pub const fn relativeFile(
        path: &'static str,
        ownership: RuntimeStorageOwnership,
        directorySegments: usize,
        fileName: &'static str,
    ) -> Self {
        Self {
            path,
            ownership,
            shape: RuntimeStoragePathShape::RelativeFile {
                directorySegments,
                fileName,
            },
        }
    }

    /// Returns whether one concrete storage path satisfies this definition.
    pub fn matches(&self, storagePath: &str) -> bool {
        match self.shape {
            RuntimeStoragePathShape::Exact => storagePath == self.path,
            RuntimeStoragePathShape::Tree => relativeRuntimeStoragePath(storagePath, self.path)
                .is_some_and(validRelativeRuntimeStoragePath),
            RuntimeStoragePathShape::RelativeFile {
                directorySegments,
                fileName,
            } => relativeRuntimeStoragePath(storagePath, self.path).is_some_and(|relativePath| {
                let segments = relativePath.split('/').collect::<Vec<_>>();
                segments.len() == directorySegments + 1
                    && segments[..directorySegments]
                        .iter()
                        .all(|segment| validRuntimeStorageSegment(segment))
                    && segments[directorySegments] == fileName
            }),
        }
    }
}

pub const RUNTIME_ROOT_DIR_PATH: &str = "runtime";
pub const RUNTIME_ROOT_PATH_PREFIX: &str = "runtime/";
pub const WORKSPACE_DIR_PATH: &str = "workspaces";
pub const WORKSPACE_ROOT_PATH_PREFIX: &str = "workspaces/";
pub const SECURE_ROOT_DIR_PATH: &str = "secure";
pub const SECURE_ROOT_PATH_PREFIX: &str = "secure/";
pub const PREFERENCES_ENCRYPTION_KEY_PATH: &str = "secure/preferences_encryption_key.json";

pub const CONFIG_PREFERENCES_DIR_PATH: &str = "runtime/config/preferences";

pub const DATA_MEMORY_CHARACTERS_USER_MARKDOWN: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::relativeFile(
        "runtime/data/memory/characters",
        RuntimeStorageOwnership::Space,
        1,
        "USER.md",
    );
pub const DATA_MEMORY_CHARACTERS_DIR_PATH: &str = DATA_MEMORY_CHARACTERS_USER_MARKDOWN.path;
pub const DATA_MEMORY_SHARED_USER_MARKDOWN: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::relativeFile(
        "runtime/data/memory/shared",
        RuntimeStorageOwnership::Space,
        1,
        "USER.md",
    );
pub const DATA_MEMORY_SHARED_DIR_PATH: &str = DATA_MEMORY_SHARED_USER_MARKDOWN.path;
pub const RUNTIME_USER_ASSETS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree("runtime/data/user_assets", RuntimeStorageOwnership::Space);
pub const RUNTIME_USER_ASSETS_DIR_PATH: &str = RUNTIME_USER_ASSETS.path;
pub const RUNTIME_CHARACTER_AVATARS_DIR_PATH: &str = "runtime/data/user_assets/character_avatars";
pub const RUNTIME_THEME_ASSETS_DIR_PATH: &str = "runtime/data/user_assets/theme";
pub const RUNTIME_SPACE_MEMBERS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree("runtime/space/members", RuntimeStorageOwnership::Space);
pub const RUNTIME_SPACE_MEMBERS_DIR_PATH: &str = RUNTIME_SPACE_MEMBERS.path;
pub const RUNTIME_SPACE_TOPOLOGY: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree("runtime/space/topology", RuntimeStorageOwnership::Space);
pub const RUNTIME_SPACE_TOPOLOGY_DIR_PATH: &str = RUNTIME_SPACE_TOPOLOGY.path;
pub const RUNTIME_SPACE_DEVICE_PROFILES: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/space/device_profiles",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH: &str = RUNTIME_SPACE_DEVICE_PROFILES.path;
pub const RUNTIME_SHARE_IMAGE: RuntimeStoragePathDefinition = RuntimeStoragePathDefinition::tree(
    "runtime/temp/share_image",
    RuntimeStorageOwnership::Ephemeral,
);
pub const RUNTIME_SHARE_IMAGE_DIR_PATH: &str = RUNTIME_SHARE_IMAGE.path;
pub const RUNTIME_WORKSPACE_VIDEO: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/temp/workspace_video",
        RuntimeStorageOwnership::Ephemeral,
    );
pub const RUNTIME_WORKSPACE_VIDEO_DIR_PATH: &str = RUNTIME_WORKSPACE_VIDEO.path;
pub const RUNTIME_COMPOSE_DSL_WEBVIEW_FILES: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/temp/compose_dsl_webview_files",
        RuntimeStorageOwnership::Ephemeral,
    );
pub const RUNTIME_COMPOSE_DSL_WEBVIEW_FILES_DIR_PATH: &str = RUNTIME_COMPOSE_DSL_WEBVIEW_FILES.path;
pub const RUNTIME_LINK_ACCESS_WEB_ASSETS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/link_access/web_access_bundle",
        RuntimeStorageOwnership::CoreNode,
    );
pub const RUNTIME_LINK_ACCESS_WEB_ASSETS_DIR_PATH: &str = RUNTIME_LINK_ACCESS_WEB_ASSETS.path;
pub const RUNTIME_LINK_ACCESS_IDENTITY: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/link_access/identity.preferences.json",
        RuntimeStorageOwnership::CoreNode,
    );
pub const RUNTIME_LINK_ACCESS_IDENTITY_PATH: &str = RUNTIME_LINK_ACCESS_IDENTITY.path;
pub const RUNTIME_LINK_ACCESS_INBOUND_SESSIONS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/link_access/inbound_sessions.preferences.json",
        RuntimeStorageOwnership::CoreNode,
    );
pub const RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH: &str =
    RUNTIME_LINK_ACCESS_INBOUND_SESSIONS.path;
pub const RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/link_access/outbound_sessions.preferences.json",
        RuntimeStorageOwnership::CoreNode,
    );
pub const RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH: &str =
    RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS.path;
pub const RUNTIME_LINK_ACCESS_PENDING_PAIRINGS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/link_access/pending_pairings.preferences.json",
        RuntimeStorageOwnership::CoreNode,
    );
pub const RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH: &str =
    RUNTIME_LINK_ACCESS_PENDING_PAIRINGS.path;
pub const RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/link_access/pending_outbound_pairings.preferences.json",
        RuntimeStorageOwnership::CoreNode,
    );
pub const RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH: &str =
    RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS.path;
pub const RUNTIME_LINK_ACCESS_HOST_CONFIG: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/link_access/host_config.preferences.json",
        RuntimeStorageOwnership::CoreNode,
    );
pub const RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH: &str = RUNTIME_LINK_ACCESS_HOST_CONFIG.path;
pub const CLIENT_RUNTIME_BOOTSTRAP: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/link/local_runtime_storage.json",
        RuntimeStorageOwnership::CoreNode,
    );
pub const CLIENT_RUNTIME_BOOTSTRAP_PATH: &str = CLIENT_RUNTIME_BOOTSTRAP.path;
pub const RUNTIME_CLIENT_LOG: RuntimeStoragePathDefinition = RuntimeStoragePathDefinition::exact(
    "runtime/logs/client.log",
    RuntimeStorageOwnership::CoreNode,
);
pub const RUNTIME_CLIENT_LOG_PATH: &str = RUNTIME_CLIENT_LOG.path;
pub const RUNTIME_SHARE_IMAGE_EXPORTS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/exports/share_image",
        RuntimeStorageOwnership::Ephemeral,
    );
pub const RUNTIME_SHARE_IMAGE_EXPORTS_DIR_PATH: &str = RUNTIME_SHARE_IMAGE_EXPORTS.path;

pub const EXTENSIONS_SKILLS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree("runtime/extensions/skills", RuntimeStorageOwnership::Space);
pub const EXTENSIONS_SKILLS_DIR_PATH: &str = EXTENSIONS_SKILLS.path;
pub const EXTENSIONS_PACKAGES_DIR_PATH: &str = "runtime/extensions/packages";
pub const EXTENSIONS_PLUGIN_CONFIGS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/extensions/plugins/configs",
        RuntimeStorageOwnership::Space,
    );
pub const EXTENSIONS_PLUGIN_CONFIGS_DIR_PATH: &str = EXTENSIONS_PLUGIN_CONFIGS.path;
pub const EXTENSIONS_MCP_DIR_PATH: &str = "runtime/extensions/mcp";

pub const RUNTIME_CLEAN_ON_EXIT: RuntimeStoragePathDefinition = RuntimeStoragePathDefinition::tree(
    "runtime/temp/clean_on_exit",
    RuntimeStorageOwnership::Ephemeral,
);
pub const RUNTIME_CLEAN_ON_EXIT_DIR_PATH: &str = RUNTIME_CLEAN_ON_EXIT.path;
pub const RUNTIME_SYNC: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree("runtime/sync", RuntimeStorageOwnership::CoreNode);
pub const RUNTIME_SYNC_DIR_PATH: &str = RUNTIME_SYNC.path;
pub const RUNTIME_SYNC_PREFERENCES_PAYLOADS_DIR_PATH: &str = "runtime/sync/preferences_payloads";
pub const RUNTIME_MODEL_CONNECTION_TEST_CACHE_DIR_PATH: &str =
    "runtime/cache/model_connection_test";
pub const RUNTIME_LOCAL_MODELS_DIR_PATH: &str = "runtime/models/local";
pub const RUNTIME_LOCAL_ENGINES_DIR_PATH: &str = "runtime/models/local/engines";
pub const RUNTIME_LOCAL_MODEL_MANIFESTS_DIR_PATH: &str = "runtime/models/local/manifests";
pub const RUNTIME_LOCAL_MODEL_REGISTRY_PATH: &str =
    "runtime/config/preferences/local_model_registry.preferences.json";
pub const RUNTIME_TOOLPKG_CACHE_DIR_PATH: &str = "runtime/cache/toolpkg";
pub const RUNTIME_TTS_AUDIO_DIR_PATH: &str = "runtime/cache/tts_audio";
pub const RUNTIME_TOOLPKG_RESOURCE_EXPORTS_DIR_PATH: &str =
    "runtime/cache/toolpkg_resource_exports";
pub const RUNTIME_TOOLPKG_RESOURCE_EXPORTS_INTERNAL_DIR_PATH: &str =
    "runtime/cache/toolpkg_resource_exports/internal";
pub const RUNTIME_IMPORTED_OPERIT1_FILES: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/imported/operit1/files",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_IMPORTED_OPERIT1_FILES_DIR_PATH: &str = RUNTIME_IMPORTED_OPERIT1_FILES.path;
pub const RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/imported/operit1/external_files",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES_DIR_PATH: &str =
    RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES.path;
pub const RUNTIME_WEBSESSION_USERSCRIPTS_STATE: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/websession/userscripts/userscripts.json",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_WEBSESSION_USERSCRIPTS_STATE_PATH: &str =
    RUNTIME_WEBSESSION_USERSCRIPTS_STATE.path;
pub const RUNTIME_WEBSESSION_BROWSER_BOOKMARKS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/websession/browser/bookmarks.json",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH: &str =
    RUNTIME_WEBSESSION_BROWSER_BOOKMARKS.path;
pub const RUNTIME_WEBSESSION_BROWSER_HISTORY: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/websession/browser/history.json",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH: &str = RUNTIME_WEBSESSION_BROWSER_HISTORY.path;
pub const RUNTIME_WEBSESSION_BROWSER_DOWNLOADS: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::exact(
        "runtime/websession/browser/downloads.json",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_WEBSESSION_BROWSER_DOWNLOADS_PATH: &str =
    RUNTIME_WEBSESSION_BROWSER_DOWNLOADS.path;
pub const RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES: RuntimeStoragePathDefinition =
    RuntimeStoragePathDefinition::tree(
        "runtime/websession/browser/download_files",
        RuntimeStorageOwnership::Space,
    );
pub const RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES_DIR_PATH: &str =
    RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES.path;

pub const EXPORTS_DIR_PATH: &str = "runtime/exports";
pub const OPERIT_LOG_PATH: &str = "runtime/logs/operit.log";
pub const TOOLPKG_LOG_PATH: &str = "runtime/logs/toolpkg.log";

pub const USER_PREFERENCES_PATH: &str =
    "runtime/config/preferences/user_preferences.preferences.json";
pub const API_PREFERENCES_PATH: &str = "runtime/config/preferences/api_settings.json";
pub const ENV_PREFERENCES_PATH: &str =
    "runtime/config/preferences/env_preferences.preferences.json";
pub const GITHUB_AUTH_PREFERENCES_PATH: &str =
    "runtime/config/preferences/github_auth_preferences.json";
pub const CHARACTER_CARDS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/character_cards.preferences.json";
pub const CHARACTER_GROUPS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/character_groups.preferences.json";
pub const PROMPT_TAGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/prompt_tags.preferences.json";
pub const SHARED_MEMORY_STORES_PREFERENCES_PATH: &str =
    "runtime/config/preferences/shared_memory_stores.preferences.json";
pub const TTS_CONFIGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/tts_configs.preferences.json";
pub const STT_CONFIGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/stt_configs.preferences.json";
pub const TOOL_PERMISSIONS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/tool_permissions.preferences.json";
pub const SKILL_VISIBILITY_PREFERENCES_PATH: &str =
    "runtime/config/preferences/skill_visibility.preferences.json";
pub const MODEL_CONFIGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/model_configs.preferences.json";
pub const FUNCTIONAL_CONFIGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/functional_configs.preferences.json";
pub const PACKAGE_MANAGER_PREFERENCES_PATH: &str =
    "runtime/config/preferences/package_manager.preferences.json";
pub const TOOLPKG_INSTALLATION_PREFERENCES_PATH: &str =
    "runtime/config/preferences/toolpkg_installation.preferences.json";
pub const DISPLAY_PREFERENCES_PATH: &str =
    "runtime/config/preferences/display_preferences.preferences.json";
pub const UI_PREFERENCES_PATH: &str = "runtime/config/preferences/ui_preferences.preferences.json";
pub const WAIFU_SETTINGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/waifu_settings.preferences.json";
pub const WAKE_WORD_PREFERENCES_PATH: &str =
    "runtime/config/preferences/wake_word_preferences.preferences.json";
pub const CUSTOM_EMOJI_SETTINGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/custom_emoji_settings.preferences.json";
pub const ANDROID_PERMISSION_PREFERENCES_PATH: &str =
    "runtime/config/preferences/android_permission_preferences.preferences.json";
pub const DATABASE_BACKUP_SETTINGS_PREFERENCES_PATH: &str =
    "runtime/config/preferences/database_backup_settings.preferences.json";
pub const PERSONA_CARD_CHAT_HISTORY_PREFERENCES_PATH: &str =
    "runtime/config/preferences/persona_card_chat_history.preferences.json";
pub const CURRENT_CHAT_ID_PREFERENCES_PATH: &str = "runtime/state/current_chat_id.preferences.json";
pub const SQLITE_DATABASE_PATH: &str = "runtime/data/database/operit2.sqlite";
pub const MCP_CONFIG_PATH: &str = "runtime/extensions/mcp/mcp_config.json";
pub const MCP_SERVER_STATUS_PATH: &str = "runtime/extensions/mcp/server_status.json";
pub const OPERIT1_SNAPSHOT_SQLITE_INSPECTION_PATH: &str =
    "runtime/temp/clean_on_exit/operit1_snapshot_import.sqlite";
pub const OPERIT1_SNAPSHOT_SQLITE_INSPECTION_WAL_PATH: &str =
    "runtime/temp/clean_on_exit/operit1_snapshot_import.sqlite-wal";
pub const OPERIT1_SNAPSHOT_SQLITE_INSPECTION_SHM_PATH: &str =
    "runtime/temp/clean_on_exit/operit1_snapshot_import.sqlite-shm";
pub const OPERIT1_SNAPSHOT_OBJECTBOX_IMPORT_PATH: &str =
    "runtime/temp/clean_on_exit/operit1_snapshot_objectbox.mdb";

/// Lists every runtime file path whose data ownership is explicitly declared.
pub const RUNTIME_STORAGE_PATH_DEFINITIONS: &[RuntimeStoragePathDefinition] = &[
    DATA_MEMORY_CHARACTERS_USER_MARKDOWN,
    DATA_MEMORY_SHARED_USER_MARKDOWN,
    RUNTIME_USER_ASSETS,
    RUNTIME_SPACE_MEMBERS,
    RUNTIME_SPACE_TOPOLOGY,
    RUNTIME_SPACE_DEVICE_PROFILES,
    RUNTIME_SHARE_IMAGE,
    RUNTIME_WORKSPACE_VIDEO,
    RUNTIME_COMPOSE_DSL_WEBVIEW_FILES,
    RUNTIME_LINK_ACCESS_WEB_ASSETS,
    RUNTIME_LINK_ACCESS_IDENTITY,
    RUNTIME_LINK_ACCESS_INBOUND_SESSIONS,
    RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS,
    RUNTIME_LINK_ACCESS_PENDING_PAIRINGS,
    RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS,
    RUNTIME_LINK_ACCESS_HOST_CONFIG,
    CLIENT_RUNTIME_BOOTSTRAP,
    RUNTIME_CLIENT_LOG,
    RUNTIME_SHARE_IMAGE_EXPORTS,
    EXTENSIONS_SKILLS,
    EXTENSIONS_PLUGIN_CONFIGS,
    RUNTIME_CLEAN_ON_EXIT,
    RUNTIME_SYNC,
    RUNTIME_IMPORTED_OPERIT1_FILES,
    RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES,
    RUNTIME_WEBSESSION_USERSCRIPTS_STATE,
    RUNTIME_WEBSESSION_BROWSER_BOOKMARKS,
    RUNTIME_WEBSESSION_BROWSER_HISTORY,
    RUNTIME_WEBSESSION_BROWSER_DOWNLOADS,
    RUNTIME_WEBSESSION_BROWSER_DOWNLOAD_FILES,
];

/// Resolves the declared owner of one concrete runtime storage path.
#[allow(non_snake_case)]
pub fn runtimeStorageOwnership(storagePath: &str) -> Result<RuntimeStorageOwnership, String> {
    let mut matchedOwnership = None;
    for definition in RUNTIME_STORAGE_PATH_DEFINITIONS {
        if !definition.matches(storagePath) {
            continue;
        }
        match matchedOwnership {
            None => matchedOwnership = Some(definition.ownership),
            Some(ownership) if ownership == definition.ownership => {}
            Some(ownership) => {
                return Err(format!(
                    "runtime storage path has conflicting ownership declarations: {storagePath} ({ownership:?}, {:?})",
                    definition.ownership
                ));
            }
        }
    }
    matchedOwnership.ok_or_else(|| {
        format!("runtime storage path has no declared data ownership: {storagePath}")
    })
}

/// Extracts one concrete descendant path below an exact registered root.
#[allow(non_snake_case)]
fn relativeRuntimeStoragePath<'a>(storagePath: &'a str, root: &str) -> Option<&'a str> {
    storagePath.strip_prefix(root)?.strip_prefix('/')
}

/// Validates every segment of one registered relative runtime path.
#[allow(non_snake_case)]
fn validRelativeRuntimeStoragePath(relativePath: &str) -> bool {
    !relativePath.is_empty() && relativePath.split('/').all(validRuntimeStorageSegment)
}

/// Validates one runtime path segment without interpreting its business meaning.
#[allow(non_snake_case)]
fn validRuntimeStorageSegment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.chars().any(|character| character == '\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies exact, tree, and relative-file definitions resolve from path metadata.
    #[test]
    fn resolves_registered_runtime_storage_ownership() {
        assert_eq!(
            runtimeStorageOwnership(RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH).unwrap(),
            RuntimeStorageOwnership::Space
        );
        assert_eq!(
            runtimeStorageOwnership("runtime/data/memory/characters/alice/USER.md").unwrap(),
            RuntimeStorageOwnership::Space
        );
        assert_eq!(
            runtimeStorageOwnership("runtime/data/user_assets/theme/custom/background.png")
                .unwrap(),
            RuntimeStorageOwnership::Space
        );
        assert_eq!(
            runtimeStorageOwnership("runtime/extensions/skills/example/SKILL.md").unwrap(),
            RuntimeStorageOwnership::Space
        );
        assert_eq!(
            runtimeStorageOwnership("runtime/extensions/plugins/configs/example/env.json").unwrap(),
            RuntimeStorageOwnership::Space
        );
        assert_eq!(
            runtimeStorageOwnership("runtime/temp/share_image/preview.png").unwrap(),
            RuntimeStorageOwnership::Ephemeral
        );
        assert_eq!(
            runtimeStorageOwnership("runtime/link_access/web_access_bundle/index.html").unwrap(),
            RuntimeStorageOwnership::CoreNode
        );
        assert_eq!(
            runtimeStorageOwnership(RUNTIME_CLIENT_LOG_PATH).unwrap(),
            RuntimeStorageOwnership::CoreNode
        );
    }

    /// Verifies undeclared and structurally invalid paths are rejected.
    #[test]
    fn rejects_unregistered_runtime_storage_paths() {
        assert!(runtimeStorageOwnership("runtime/data/memory/characters/alice/notes.md").is_err());
        assert!(runtimeStorageOwnership("runtime/data/memory/characters/a/b/USER.md").is_err());
        assert!(runtimeStorageOwnership("runtime/extensions/skills/../secret.txt").is_err());
        assert!(runtimeStorageOwnership("runtime/unclassified/data.bin").is_err());
    }
}
