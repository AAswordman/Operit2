use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::convert::TryInto;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use chrono::TimeZone;
use operit_host_api::{
    RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageHost, RuntimeStorageWriteHost,
    RuntimeStorageWriteSession, SqliteRow, SqliteValue,
};
use operit_store::PreferencesDataStore::{
    mutableStateFlow, stringPreferencesKey, MutableStateFlow, PreferencesDataStore, StateFlow,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::AppLogger::AppLogger;
use operit_util::RuntimeStorageLayout::{
    ANDROID_PERMISSION_PREFERENCES_PATH, API_PREFERENCES_PATH,
    CUSTOM_EMOJI_SETTINGS_PREFERENCES_PATH, DATABASE_BACKUP_SETTINGS_PREFERENCES_PATH,
    DATA_MEMORY_SHARED_DIR_PATH, DISPLAY_PREFERENCES_PATH, GITHUB_AUTH_PREFERENCES_PATH,
    OPERIT1_SNAPSHOT_OBJECTBOX_IMPORT_PATH, OPERIT1_SNAPSHOT_SQLITE_INSPECTION_PATH,
    OPERIT1_SNAPSHOT_SQLITE_INSPECTION_SHM_PATH, OPERIT1_SNAPSHOT_SQLITE_INSPECTION_WAL_PATH,
    PERSONA_CARD_CHAT_HISTORY_PREFERENCES_PATH, RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES_DIR_PATH,
    RUNTIME_IMPORTED_OPERIT1_FILES_DIR_PATH, UI_PREFERENCES_PATH, USER_PREFERENCES_PATH,
    WAIFU_SETTINGS_PREFERENCES_PATH, WAKE_WORD_PREFERENCES_PATH, WORKSPACE_DIR_PATH,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::data::archive::ArchiveSource::ArchiveSource;
use crate::data::backup::Operit1LmdbReader::visitLmdbRecords;
use crate::data::backup::Operit1RoomSchemaMigration::{
    prepareOperit1RoomImport, Operit1ToOperit2ChatArchiveBridge,
};
use crate::data::backup::Operit1SnapshotArchive::{
    isDataStoreEntry as isArchiveDataStoreEntry, validateRelativePath, Operit1PreferenceValue,
    Operit1SnapshotArchive, Operit1SnapshotEntry,
};
use crate::data::backup::Operit1ThinkingMigration::convert_thinking_configurations;

use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::preferences::SharedMemoryStoreManager::SharedMemoryStoreManager;
use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use operit_model::ApiKeyInfo::{ApiKeyAvailabilityStatus, ApiKeyInfo};
use operit_model::CharacterCard::{
    CharacterCard, CharacterCardChatModelBindingMode, CharacterCardMemoryBindingMode,
    CharacterCardToolAccessConfig,
};
use operit_model::CharacterGroupCard::{CharacterGroupCard, GroupMemberConfig};
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use operit_model::FunctionType::FunctionType;
use operit_model::MemoryExportModel::{
    ImportStrategy, MemoryExportData, SerializableLink, SerializableMemory,
};
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::ModelCatalog::ModelCatalog;
use operit_model::ModelConfigData::{
    ApiProviderType, ModelCapabilities, ModelConfigDefaults, ModelContextSpec, ModelProfile,
    ModelRequestSpec, ModelSummarySettings, ProviderProfile,
};
use operit_model::ModelParameter::{CustomParameterData, ModelParameter, ParameterCategory};
use operit_model::OperitChatArchive::{
    OperitArchivedChat, OperitArchivedMessage, OperitArchivedMessageVariant, OperitChatArchive,
    ARCHIVE_TYPE, CURRENT_FORMAT_VERSION,
};
use operit_model::PromptTag::{PromptTag, TagType};
use operit_model::StandardModelParameters::StandardModelParameters;
use operit_model::TtsConfig::{
    TtsConfig, TtsHttpHeader, TtsHttpResponsePipelineStep, TtsProviderType,
};
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use operit_store::repository::MemoryRepository::MemoryRepository;
use operit_store::repository::UsageStatisticsStore::{TokenStatsModel, UsageStatisticsStore};
use operit_util::OperitPaths::{sanitizeMemoryOwnerId, sharedMemoryOwnerKey};

const FORMAT_VERSION: i32 = 1;
const ENTRY_MANIFEST: &str = "manifest.json";
const ENTRY_MODEL_CONFIGS: &str = "payload/files/datastore/model_configs.preferences_pb";
const ENTRY_FUNCTIONAL_CONFIGS: &str = "payload/files/datastore/functional_configs.preferences_pb";
const ENTRY_CHARACTER_CARDS: &str = "payload/files/datastore/character_cards.preferences_pb";
const ENTRY_CHARACTER_GROUPS: &str = "payload/files/datastore/character_groups.preferences_pb";
const ENTRY_PROMPT_TAGS: &str = "payload/files/datastore/prompt_tags.preferences_pb";
const ENTRY_SPEECH_SERVICES: &str =
    "payload/files/datastore/speech_services_preferences.preferences_pb";
const ENTRY_USER_PREFERENCES: &str = "payload/files/datastore/user_preferences.preferences_pb";
const ENTRY_DATABASE: &str = "payload/databases/app_database";
const ENTRY_DATABASE_WAL: &str = "payload/databases/app_database-wal";
const ENTRY_DATABASE_SHM: &str = "payload/databases/app_database-shm";
const ENTRY_OBJECTBOX_DEFAULT_DATA: &str = "payload/files/objectbox/data.mdb";
const ENTRY_DATASTORE_PREFIX: &str = "payload/files/datastore/";
const ENTRY_FILES_PREFIX: &str = "payload/files/";
const ENTRY_WORKSPACE_FILES_PREFIX: &str = "payload/files/workspace/";
const ENTRY_EXTERNAL_FILES_PREFIX: &str = "payload/external_files/";
const KEY_CONFIG_LIST: &str = "config_list";
const KEY_FUNCTION_CONFIG_MAPPING: &str = "function_config_mapping";
const KEY_ACTIVE_MEMORY_SPACE_ID: &str = "active_memory_space_id";
const KEY_MEMORY_SPACE_LIST: &str = "memory_space_list";
const KEY_MEMORY_SPACE_PREFIX: &str = "memory_space_";
const OPERIT1_DEFAULT_PROFILE_ID: &str = "default";
const OPERIT1_SHARED_MEMORY_STORE_ID_PREFIX: &str = "operit1-profile-";
const ARCHIVE_ENTRY_COPY_BUFFER_BYTES: usize = 256 * 1024;
const OPERIT1_PROGRESS_REPORT_INTERVAL_MS: i64 = 250;
const OPERIT1_INTERNAL_FILES_PREFIXES: [&str; 4] = [
    "/data/user/0/com.ai.assistance.operit/files/",
    "/data/data/com.ai.assistance.operit/files/",
    "/data/user/0/com.ai.assistance.operit.debug/files/",
    "/data/data/com.ai.assistance.operit.debug/files/",
];
const OPERIT1_EXTERNAL_DOWNLOAD_PREFIX: &str = "/storage/emulated/0/Download/Operit/";
const OPERIT1_DEFAULT_AI_AVATAR_URI: &str = "file:///android_asset/operit.png";
const OPERIT1_OBJECTBOX_KEY_MEMORY: [u8; 4] = [0x18, 0x00, 0x00, 0x10];
const OPERIT1_OBJECTBOX_KEY_LINK: [u8; 4] = [0x18, 0x00, 0x00, 0x14];
const OPERIT1_OBJECTBOX_KEY_TAG: [u8; 4] = [0x18, 0x00, 0x00, 0x1c];
const OPERIT1_OBJECTBOX_KEY_MEMORY_TAG_RELATION: [u8; 4] = [0x20, 0x00, 0x00, 0x20];

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
/// Preview of an Operit1 snapshot before import.
pub struct Operit1SnapshotPreview {
    pub formatVersion: i32,
    pub packageName: String,
    pub createdAt: i64,
    pub modelConfig: Operit1ModelConfigSnapshotPreview,
    pub datastoreFiles: Vec<Operit1DataStoreFilePreview>,
    pub chatCount: i32,
    pub messageCount: i32,
    pub importedFileCount: i32,
    pub importedExternalFileCount: i32,
    pub detectedDomains: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
/// Summary for a datastore preference file inside an Operit1 snapshot.
pub struct Operit1DataStoreFilePreview {
    pub fileName: String,
    pub keyCount: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
/// Counters returned after importing a full Operit1 snapshot.
pub struct Operit1SnapshotImportResult {
    pub modelConfig: Operit1ModelConfigImportResult,
    pub importedDatastoreFiles: i32,
    pub importedDatastoreKeys: i32,
    pub importedChats: i32,
    pub importedMessages: i32,
    pub importedTokenUsageRecords: i32,
    pub importedTokenStatsModels: i32,
    pub importedMemories: i32,
    pub importedMemoryLinks: i32,
    pub importedFiles: i32,
    pub importedExternalFiles: i32,
    pub importedWorkspaces: i32,
    pub importedWorkspaceFiles: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
/// Progress state published while an Operit1 snapshot import is running.
pub struct Operit1SnapshotImportProgress {
    pub stage: String,
    pub title: String,
    pub detail: String,
    pub progress: f32,
    pub active: bool,
}

impl Operit1SnapshotImportProgress {
    fn idle() -> Self {
        Self {
            stage: "idle".to_string(),
            title: "等待导入".to_string(),
            detail: "选择 Operit1 快照后开始迁移。".to_string(),
            progress: 0.0,
            active: false,
        }
    }

    fn stage(stage: &str, title: &str, detail: &str, progress: f32) -> Self {
        Self {
            stage: stage.to_string(),
            title: title.to_string(),
            detail: detail.to_string(),
            progress,
            active: true,
        }
    }

    fn completed(result: &Operit1SnapshotImportResult) -> Self {
        Self {
            stage: "completed".to_string(),
            title: "导入完成".to_string(),
            detail: format!(
                "已迁移 {} 个聊天、{} 条消息、{} 条统计记录、{} 条记忆和 {} 个资源文件。",
                result.importedChats,
                result.importedMessages,
                result.importedTokenUsageRecords,
                result.importedMemories,
                result.importedFiles + result.importedExternalFiles + result.importedWorkspaceFiles
            ),
            progress: 1.0,
            active: false,
        }
    }
}

static OPERIT1_SNAPSHOT_IMPORT_PROGRESS_FLOW: OnceLock<
    MutableStateFlow<Operit1SnapshotImportProgress>,
> = OnceLock::new();

fn operit1SnapshotImportProgressFlow() -> &'static MutableStateFlow<Operit1SnapshotImportProgress> {
    OPERIT1_SNAPSHOT_IMPORT_PROGRESS_FLOW
        .get_or_init(|| mutableStateFlow(Operit1SnapshotImportProgress::idle()))
}

/// Observes Operit1 snapshot import progress.
pub fn observeOperit1SnapshotImportProgress() -> StateFlow<Operit1SnapshotImportProgress> {
    operit1SnapshotImportProgressFlow().asStateFlow()
}

/// Publishes Operit1 snapshot import progress.
pub fn publishOperit1SnapshotImportProgress(progress: Operit1SnapshotImportProgress) {
    AppLogger::i(
        "Operit1SnapshotImport",
        &format!(
            "stage={} progress={:.0}% active={} title={} detail={}",
            progress.stage,
            progress.progress * 100.0,
            progress.active,
            progress.title,
            progress.detail,
        ),
    );
    operit1SnapshotImportProgressFlow().set_value(progress);
}

/// Publishes progress for a stage whose work is measured in completed units.
fn publishOperit1SnapshotCountedProgress(
    stage: &str,
    title: &str,
    detail: String,
    stageStart: f32,
    stageEnd: f32,
    completed: usize,
    total: usize,
) {
    let stageFraction = completed as f32 / total.max(1) as f32;
    let progress = stageStart + (stageEnd - stageStart) * stageFraction;
    publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
        stage, title, &detail, progress,
    ));
}

/// Limits high-frequency counted progress reports while preserving first and final updates.
struct Operit1SnapshotProgressThrottle {
    hasPublished: bool,
    lastPublishedAt: i64,
}

impl Operit1SnapshotProgressThrottle {
    /// Creates a progress-report throttle with no previously published unit.
    fn new() -> Self {
        Self {
            hasPublished: false,
            lastPublishedAt: 0,
        }
    }

    /// Returns whether a completed-unit update should be published now.
    fn shouldPublish(&mut self, completed: usize, total: usize) -> bool {
        let now = currentTimeMillis();
        let mustPublish = completed >= total
            || !self.hasPublished
            || now.saturating_sub(self.lastPublishedAt) >= OPERIT1_PROGRESS_REPORT_INTERVAL_MS;
        if mustPublish {
            self.hasPublished = true;
            self.lastPublishedAt = now;
        }
        mustPublish
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
/// Preview of model configuration data inside an Operit1 snapshot.
pub struct Operit1ModelConfigSnapshotPreview {
    pub formatVersion: i32,
    pub packageName: String,
    pub createdAt: i64,
    pub configs: Vec<Operit1ModelConfigPreview>,
    pub chatConfigId: Option<String>,
    pub chatModelId: Option<String>,
    pub chatModelIndex: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
/// Preview of one Operit1 model configuration entry.
pub struct Operit1ModelConfigPreview {
    pub configId: String,
    pub name: String,
    pub providerTypeId: String,
    pub providerDisplayName: String,
    pub endpoint: String,
    pub modelIds: Vec<String>,
    pub selectedModelId: Option<String>,
    pub selectedModelIndex: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
/// Result returned after importing Operit1 model configuration.
pub struct Operit1ModelConfigImportResult {
    pub providerId: String,
    pub providerTypeId: String,
    pub providerName: String,
    pub modelId: String,
    pub importedModelCount: i32,
    pub chatBindingUpdated: bool,
    pub skippedFields: Vec<String>,
}

#[derive(Clone)]
/// Imports Operit1 backup snapshots into the current runtime storage layout.
pub struct Operit1SnapshotImportManager {
    paths: RuntimeStorePaths,
    storageHost: Arc<dyn RuntimeStorageHost>,
    storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
    sqliteHost: Arc<dyn RuntimeSqliteHost>,
}

impl Operit1SnapshotImportManager {
    /// Creates an importer targeting one runtime owner's portable storage hosts.
    pub fn new(
        paths: RuntimeStorePaths,
        storageHost: Arc<dyn RuntimeStorageHost>,
        storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
        sqliteHost: Arc<dyn RuntimeSqliteHost>,
    ) -> Self {
        Self {
            paths,
            storageHost,
            storageWriteHost,
            sqliteHost,
        }
    }

    #[allow(non_snake_case)]
    /// Reads an Operit1 snapshot from a range-readable source and returns preview metadata.
    pub(crate) fn inspectSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<Operit1SnapshotPreview, String> {
        let result = (|| {
            let parsed = ParsedOperit1Snapshot::fromSource(source)?;
            let databaseCounts = self.databaseCounts(&parsed)?;
            parsed.preview(databaseCounts)
        })();
        match &result {
            Ok(preview) => AppLogger::i(
                "Operit1SnapshotImport",
                &format!(
                    "inspection completed formatVersion={} configs={} chats={} messages={} files={}",
                    preview.formatVersion,
                    preview.modelConfig.configs.len(),
                    preview.chatCount,
                    preview.messageCount,
                    preview.importedFileCount + preview.importedExternalFileCount,
                ),
            ),
            Err(error) => AppLogger::e(
                "Operit1SnapshotImport",
                &format!("inspection failed: {error}"),
            ),
        };
        result
    }

    #[allow(non_snake_case)]
    /// Imports a full Operit1 snapshot from a range-readable source into runtime storage.
    pub(crate) fn importSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<Operit1SnapshotImportResult, String> {
        AppLogger::i("Operit1SnapshotImport", "full snapshot import started");
        let result = self.importSnapshotSourceInner(source);
        match &result {
            Ok(imported) => AppLogger::i(
                "Operit1SnapshotImport",
                &format!(
                    "full snapshot import completed chats={} messages={} memories={} files={}",
                    imported.importedChats,
                    imported.importedMessages,
                    imported.importedMemories,
                    imported.importedFiles
                        + imported.importedExternalFiles
                        + imported.importedWorkspaceFiles,
                ),
            ),
            Err(error) => AppLogger::e(
                "Operit1SnapshotImport",
                &format!("full snapshot import failed: {error}"),
            ),
        };
        result
    }

    /// Runs the full Operit1 snapshot migration after its start has been logged.
    fn importSnapshotSourceInner(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<Operit1SnapshotImportResult, String> {
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "parse",
            "解析快照",
            "正在读取清单、配置和数据索引。",
            0.08,
        ));
        let parsed = ParsedOperit1Snapshot::fromSource(source)?;
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "model_config",
            "迁移模型配置",
            "正在写入供应商、密钥和默认聊天模型。",
            0.22,
        ));
        let selected = parsed.selectedChatConfig()?;
        let selectedModelId = selected
            .selectedModelId
            .clone()
            .ok_or_else(|| "Operit1 快照里的聊天模型索引没有对应模型".to_string())?;
        let modelConfig =
            self.importModelConfigFromParsed(&parsed, selected.configId.clone(), selectedModelId)?;
        let fileImportPlan = SnapshotFileImportPlan::new();
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "structured_preferences",
            "迁移角色和语音",
            "正在写入角色卡、提示词、角色组和 TTS 配置。",
            0.36,
        ));
        self.importStructuredPreferences(&parsed, &fileImportPlan)?;
        self.importUserMarkdownPreferences(&parsed)?;
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "preferences",
            "迁移偏好设置",
            "正在写入主题、功能映射和数据存储偏好。",
            0.50,
        ));
        let (importedDatastoreFiles, importedDatastoreKeys) =
            self.importDataStorePreferences(&parsed, &fileImportPlan)?;
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "chats",
            "迁移聊天记录",
            "正在导入历史会话和消息。",
            0.64,
        ));
        let (importedChats, importedMessages) =
            self.importChatDatabase(&parsed, &fileImportPlan)?;
        let (importedTokenUsageRecords, importedTokenStatsModels) =
            self.importTokenStatistics(&parsed)?;
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "memory",
            "迁移记忆库",
            "正在转换 Operit1 记忆和关联关系。",
            0.78,
        ));
        let (importedMemories, importedMemoryLinks) = self.importObjectBoxMemoryStore(&parsed)?;
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "files",
            "迁移资源文件",
            "正在复制工作区、附件和外部资源。",
            0.90,
        ));
        let fileImportResult = self.importSnapshotFiles(&parsed)?;
        let result = Operit1SnapshotImportResult {
            modelConfig,
            importedDatastoreFiles,
            importedDatastoreKeys,
            importedChats,
            importedMessages,
            importedTokenUsageRecords,
            importedTokenStatsModels,
            importedMemories,
            importedMemoryLinks,
            importedFiles: fileImportResult.importedFiles,
            importedExternalFiles: fileImportResult.importedExternalFiles,
            importedWorkspaces: fileImportResult.importedWorkspaces,
            importedWorkspaceFiles: fileImportResult.importedWorkspaceFiles,
        };
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::completed(&result));
        Ok(result)
    }

    #[allow(non_snake_case)]
    fn importStructuredPreferences(
        &self,
        parsed: &ParsedOperit1Snapshot,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<(), String> {
        let paths = self.paths.clone();
        let promptTags = buildOperit2PromptTags(parsed)?;
        if !promptTags.is_empty()
            || parsed
                .archive
                .datastorePreferences
                .contains_key(ENTRY_CHARACTER_CARDS)
        {
            let cards = buildOperit2CharacterCards(parsed, fileImportPlan)?;
            let backup = serde_json::json!({
                "characterCards": cards,
                "promptTags": promptTags,
            });
            CharacterCardManager::new(paths.clone())
                .importAllCharacterCardsFromBackupContent(
                    &serde_json::to_string(&backup).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("导入 Operit1 角色卡失败：{error}"))?;
        }

        let groups = buildOperit2CharacterGroups(parsed)?;
        if !groups.is_empty() {
            let backup = serde_json::json!({
                "characterGroups": groups,
            });
            CharacterGroupCardManager::new(paths.clone())
                .importAllCharacterGroupsFromBackupContent(
                    &serde_json::to_string(&backup).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("导入 Operit1 角色组失败：{error}"))?;
        }

        if let Some(config) = buildOperit2TtsConfig(parsed)? {
            let manager = TtsConfigManager::new(paths);
            importOperit1TtsConfig(&manager, config)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn importUserMarkdownPreferences(&self, parsed: &ParsedOperit1Snapshot) -> Result<(), String> {
        let profiles = buildOperit1UserPreferenceProfiles(parsed)?;
        let cardBindings = collectOperit1CharacterMemoryProfileBindings(parsed)?;
        let profileIds = cardBindings.values().cloned().collect::<BTreeSet<String>>();
        for profileId in profileIds {
            let profile = profiles
                .get(&profileId)
                .ok_or_else(|| format!("Operit1 角色卡绑定了不存在的用户偏好：{profileId}"))?;
            let markdown = buildOperit2UserMarkdown(profile)?;
            appendSharedUserMarkdown(self.storageHost.as_ref(), &profileId, &markdown)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Reads model configuration preview metadata from a range-readable Operit1 source.
    pub(crate) fn inspectModelConfigSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<Operit1ModelConfigSnapshotPreview, String> {
        ParsedOperit1Snapshot::fromSource(source)?.modelConfigPreview()
    }

    #[allow(non_snake_case)]
    /// Imports one selected model configuration from a range-readable Operit1 source.
    pub(crate) fn importModelConfigSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
        configId: String,
        modelId: String,
    ) -> Result<Operit1ModelConfigImportResult, String> {
        let result = (|| {
            let parsed = ParsedOperit1Snapshot::fromSource(source)?;
            self.importModelConfigFromParsed(&parsed, configId, modelId)
        })();
        if let Err(error) = &result {
            AppLogger::e(
                "Operit1SnapshotImport",
                &format!("model configuration import failed: {error}"),
            );
        }
        result
    }

    #[allow(non_snake_case)]
    fn importModelConfigFromParsed(
        &self,
        parsed: &ParsedOperit1Snapshot,
        configId: String,
        modelId: String,
    ) -> Result<Operit1ModelConfigImportResult, String> {
        let config = parsed.configById(&configId)?;
        let modelIds = splitModelIds(&config.modelName);
        if !modelIds.iter().any(|current| current == &modelId) {
            return Err(format!(
                "模型配置「{}」不包含模型：{}",
                config.name, modelId
            ));
        }
        let providerBinding = config.providerBinding()?;
        let endpoint = config.apiEndpoint.trim().to_string();
        if endpoint.is_empty() {
            return Err(format!("模型配置「{}」缺少服务地址", config.name));
        }

        let mut provider = ProviderProfile::new(
            ModelConfigDefaults::DEFAULT_PROVIDER_ID.to_string(),
            providerBinding.displayName,
            providerBinding.providerType,
            endpoint,
        );
        provider.providerTypeId = providerBinding.providerTypeId;
        provider.apiKey = config.apiKey.clone();
        provider.useMultipleApiKeys = config.useMultipleApiKeys;
        provider.apiKeyPool = config
            .apiKeyPool
            .iter()
            .map(Operit1ApiKeyInfo::toApiKeyInfo)
            .collect();
        provider.currentKeyIndex = config.currentKeyIndex;
        provider.keyRotationMode = config.keyRotationMode.clone();
        provider.customHeaders = config.customHeaders.clone();
        provider.requestLimitPerMinute = config.requestLimitPerMinute;
        provider.maxConcurrentRequests = config.maxConcurrentRequests;
        provider.models = modelIds
            .iter()
            .map(|currentModelId| buildModelProfile(currentModelId, &config))
            .collect::<Result<Vec<_>, String>>()?;

        let modelConfigManager = ModelConfigManager::new(self.paths.runtime_dir().to_path_buf());
        modelConfigManager
            .replaceDefaultProviderProfile(provider.clone())
            .map_err(|error| error.to_string())?;

        let functionalConfigManager =
            FunctionalConfigManager::new(self.paths.runtime_dir().to_path_buf());
        functionalConfigManager
            .setModelForFunction(
                FunctionType::CHAT,
                ModelConfigDefaults::DEFAULT_PROVIDER_ID.to_string(),
                modelId.clone(),
            )
            .map_err(|error| error.to_string())?;

        Ok(Operit1ModelConfigImportResult {
            providerId: provider.id,
            providerTypeId: provider.providerTypeId,
            providerName: provider.name,
            modelId,
            importedModelCount: provider.models.len() as i32,
            chatBindingUpdated: true,
            skippedFields: vec![
                "contextLength".to_string(),
                "summaryCustomRules".to_string(),
                "enableGoogleSearch".to_string(),
                "enableClaude1hPromptCache".to_string(),
            ],
        })
    }

    #[allow(non_snake_case)]
    /// Imports mapped Operit1 DataStore preferences and reports each written file.
    fn importDataStorePreferences(
        &self,
        parsed: &ParsedOperit1Snapshot,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<(i32, i32), String> {
        let paths = self.paths.clone();
        let mappings = datastorePreferenceMappings(&paths);
        let mappedFileCount = mappings
            .iter()
            .filter(|(entryName, _)| parsed.archive.datastorePreferences.contains_key(*entryName))
            .count();
        let mut fileCount = 0;
        let mut keyCount = 0;
        for (entryName, filePath) in mappings {
            let Some(preferences) = parsed.archive.datastorePreferences.get(&entryName) else {
                continue;
            };
            let encodedPreferences = preferences
                .iter()
                .map(|(key, value)| value.toTargetPreferenceEntry(key, fileImportPlan))
                .collect::<Result<Vec<_>, _>>()?;
            let store = PreferencesDataStore::new(filePath);
            store
                .edit(|target| {
                    for (key, value) in &encodedPreferences {
                        target.set(&stringPreferencesKey(key), value.clone());
                    }
                })
                .map_err(|error| error.to_string())?;
            fileCount += 1;
            keyCount += preferences.len() as i32;
            publishOperit1SnapshotCountedProgress(
                "preferences",
                "迁移偏好设置",
                format!(
                    "已写入 {fileCount}/{mappedFileCount} 个偏好文件，累计 {keyCount} 个配置项。"
                ),
                0.50,
                0.64,
                fileCount as usize,
                mappedFileCount,
            );
        }
        if mappedFileCount == 0 {
            publishOperit1SnapshotCountedProgress(
                "preferences",
                "迁移偏好设置",
                "快照没有可迁移的数据存储偏好文件。".to_string(),
                0.50,
                0.64,
                0,
                0,
            );
        }
        Ok((fileCount, keyCount))
    }

    #[allow(non_snake_case)]
    /// Imports Operit1 chats while reporting parsed messages and persisted chats.
    fn importChatDatabase(
        &self,
        parsed: &ParsedOperit1Snapshot,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<(i32, i32), String> {
        self.withStagedOperit1ChatDatabase(parsed, |connection, archiveBridge| {
            let (totalChatCount, totalMessageCount) = operit1ChatDatabaseCounts(connection)?;
            let mut parsedMessageCount = 0usize;
            let mut parsedMessageProgressThrottle = Operit1SnapshotProgressThrottle::new();
            let mut onMessageParsed = |parsedChatCount: usize| {
                parsedMessageCount += 1;
                if parsedMessageProgressThrottle
                    .shouldPublish(parsedMessageCount, totalMessageCount as usize)
                {
                    publishOperit1SnapshotCountedProgress(
                        "chats",
                        "迁移聊天记录",
                        format!(
                            "正在解析聊天：{parsedChatCount}/{totalChatCount} 个会话，{parsedMessageCount}/{totalMessageCount} 条消息。"
                        ),
                        0.64,
                        0.71,
                        parsedMessageCount,
                        totalMessageCount as usize,
                    );
                }
            };
            let archive = buildChatArchiveFromOperit1ToOperit2DatabaseBridge(
                archiveBridge,
                connection,
                fileImportPlan,
                &mut onMessageParsed,
            )?;
            let chatCount = archive.chats.len() as i32;
            let messageCount = archive
                .chats
                .iter()
                .map(|chat| chat.messages.len() as i32)
                .sum();
            publishOperit1SnapshotCountedProgress(
                "chats",
                "迁移聊天记录",
                format!("解析完成，正在写入 {chatCount} 个会话和 {messageCount} 条消息。"),
                0.64,
                0.71,
                1,
                1,
            );
            let manager =
                ChatHistoryManager::new(self.paths.clone()).map_err(|error| error.to_string())?;
            let mut persistedChatProgressThrottle = Operit1SnapshotProgressThrottle::new();
            manager
                .importChatArchiveWithProgress(archive, |persistedChats, totalChats, _| {
                    if persistedChatProgressThrottle.shouldPublish(persistedChats, totalChats) {
                        publishOperit1SnapshotCountedProgress(
                            "chats",
                            "迁移聊天记录",
                            format!(
                                "正在写入聊天：{persistedChats}/{totalChats} 个会话，{messageCount} 条消息已解析。"
                            ),
                            0.71,
                            0.78,
                            persistedChats,
                            totalChats,
                        );
                    }
                })
                .map_err(|error| error.to_string())?;
            if chatCount == 0 {
                publishOperit1SnapshotCountedProgress(
                    "chats",
                    "迁移聊天记录",
                    "快照没有可导入的聊天记录。".to_string(),
                    0.71,
                    0.78,
                    0,
                    0,
                );
            }
            Ok((chatCount, messageCount))
        })
    }

    #[allow(non_snake_case)]
    /// Imports the OP1 v21 token ledger and pricing identities without deriving usage from chats.
    fn importTokenStatistics(&self, parsed: &ParsedOperit1Snapshot) -> Result<(i32, i32), String> {
        self.withStagedOperit1ChatDatabase(parsed, |connection, bridge| {
            if bridge != Operit1ToOperit2ChatArchiveBridge::Operit1RoomV21ToOperit2SqliteV27 {
                return Ok((0, 0));
            }
            let usageRows = connection
                .query(
                    r#"
                    SELECT importKey, occurredAtMs, configId, provider, model, requestCount,
                        uncachedInputTokens, cachedInputTokens, cacheWriteTokens,
                        totalInputTokens, outputTokens
                    FROM token_usage_records
                    ORDER BY id ASC
                    "#,
                    Vec::new(),
                )
                .map_err(|error| error.to_string())?;
            let pricingRows = connection
                .query(
                    r#"
                    SELECT configId, provider, model, billingMode, currency,
                        inputPricePerMillion, cachedInputPricePerMillion,
                        cacheWritePricePerMillion, outputPricePerMillion, pricePerRequest
                    FROM token_stats_models
                    ORDER BY provider ASC, model ASC, configId ASC
                    "#,
                    Vec::new(),
                )
                .map_err(|error| error.to_string())?;
            let statisticsStore = UsageStatisticsStore::new();
            for row in &usageRows {
                statisticsStore.recordTokenUsage(
                    sqliteRowOptionalString(row, 0, "token_usage_records.importKey")?,
                    sqliteRowOptionalI64(row, 1, "token_usage_records.occurredAtMs")?,
                    sqliteRowString(row, 2, "token_usage_records.configId")?,
                    sqliteRowString(row, 3, "token_usage_records.provider")?,
                    sqliteRowString(row, 4, "token_usage_records.model")?,
                    sqliteRowI64(row, 5, "token_usage_records.requestCount")?,
                    sqliteRowOptionalI64(row, 6, "token_usage_records.uncachedInputTokens")?,
                    sqliteRowOptionalI64(row, 7, "token_usage_records.cachedInputTokens")?,
                    sqliteRowOptionalI64(row, 8, "token_usage_records.cacheWriteTokens")?,
                    sqliteRowOptionalI64(row, 9, "token_usage_records.totalInputTokens")?,
                    sqliteRowOptionalI64(row, 10, "token_usage_records.outputTokens")?,
                )?;
            }
            for row in &pricingRows {
                statisticsStore.upsertTokenStatsModel(TokenStatsModel {
                    configId: sqliteRowString(row, 0, "token_stats_models.configId")?,
                    provider: sqliteRowString(row, 1, "token_stats_models.provider")?,
                    model: sqliteRowString(row, 2, "token_stats_models.model")?,
                    billingMode: sqliteRowOptionalString(row, 3, "token_stats_models.billingMode")?,
                    currency: sqliteRowOptionalString(row, 4, "token_stats_models.currency")?,
                    inputPricePerMillion: sqliteRowOptionalF64(
                        row,
                        5,
                        "token_stats_models.inputPricePerMillion",
                    )?,
                    cachedInputPricePerMillion: sqliteRowOptionalF64(
                        row,
                        6,
                        "token_stats_models.cachedInputPricePerMillion",
                    )?,
                    cacheWritePricePerMillion: sqliteRowOptionalF64(
                        row,
                        7,
                        "token_stats_models.cacheWritePricePerMillion",
                    )?,
                    outputPricePerMillion: sqliteRowOptionalF64(
                        row,
                        8,
                        "token_stats_models.outputPricePerMillion",
                    )?,
                    pricePerRequest: sqliteRowOptionalF64(
                        row,
                        9,
                        "token_stats_models.pricePerRequest",
                    )?,
                })?;
            }
            Ok((usageRows.len() as i32, pricingRows.len() as i32))
        })
    }

    #[allow(non_snake_case)]
    /// Imports workspace, internal, and external snapshot files one entry at a time.
    fn importSnapshotFiles(
        &self,
        parsed: &ParsedOperit1Snapshot,
    ) -> Result<Operit1SnapshotFileImportResult, String> {
        let copyPlan = buildSnapshotFileCopyPlan(parsed)?;
        let totalFileCount = copyPlan.items.len();
        let sourceEntries = copyPlan
            .items
            .iter()
            .map(|item| item.sourceEntry.clone())
            .collect::<Vec<_>>();
        let mut copiedFileCount = 0usize;
        let mut copyBuffer = vec![0; ARCHIVE_ENTRY_COPY_BUFFER_BYTES];
        let mut fileProgressThrottle = Operit1SnapshotProgressThrottle::new();
        parsed
            .archive
            .copyEntriesTo(&sourceEntries, |index, entryName, reader| {
                let item = copyPlan.items.get(index).ok_or_else(|| {
                    format!("Operit1 snapshot copy plan is missing entry index: {index}")
                })?;
                writeArchiveReaderToStorage(
                    self.storageWriteHost.as_ref(),
                    reader,
                    &item.targetPath,
                    &mut copyBuffer,
                )?;
                copiedFileCount += 1;
                if fileProgressThrottle.shouldPublish(copiedFileCount, totalFileCount) {
                    publishOperit1SnapshotCountedProgress(
                        "files",
                        "迁移资源文件",
                        format!(
                            "正在复制资源：{copiedFileCount}/{totalFileCount} 个文件（{entryName}）。"
                        ),
                        0.90,
                        1.0,
                        copiedFileCount,
                        totalFileCount,
                    );
                }
                Ok(())
            })?;
        if totalFileCount == 0 {
            publishOperit1SnapshotCountedProgress(
                "files",
                "迁移资源文件",
                "快照没有可迁移的资源文件。".to_string(),
                0.90,
                1.0,
                0,
                0,
            );
        }
        Ok(Operit1SnapshotFileImportResult {
            importedFiles: copyPlan.importedFiles,
            importedExternalFiles: copyPlan.importedExternalFiles,
            importedWorkspaces: copyPlan.workspaceIds.len() as i32,
            importedWorkspaceFiles: copyPlan.importedWorkspaceFiles,
        })
    }

    #[allow(non_snake_case)]
    /// Imports every Operit1 ObjectBox memory profile and reports each completed profile.
    fn importObjectBoxMemoryStore(
        &self,
        parsed: &ParsedOperit1Snapshot,
    ) -> Result<(i32, i32), String> {
        let result = (|| {
            let paths = self.paths.clone();
            let sharedMemoryStoreManager = SharedMemoryStoreManager::new(paths);
            let profiles = collectOperit1MemoryProfileIds(parsed)?;
            let profileCount = profiles.len();
            let mut totalMemoryCount = 0;
            let mut totalLinkCount = 0;
            for (profileIndex, profileId) in profiles.into_iter().enumerate() {
                let entry = operit1ObjectBoxEntryForProfile(&profileId);
                let entryLength = parsed
                    .archive
                    .entries
                    .get(&entry)
                    .ok_or_else(|| format!("Operit1 记忆库条目不存在：{entry}"))?
                    .uncompressedSize;
                let exportData = self.withStagedArchiveEntry(
                    parsed,
                    &entry,
                    OPERIT1_SNAPSHOT_OBJECTBOX_IMPORT_PATH,
                    || {
                        buildMemoryExportDataFromOperit1ObjectBox(
                            self.storageHost.as_ref(),
                            OPERIT1_SNAPSHOT_OBJECTBOX_IMPORT_PATH,
                            entryLength,
                        )
                    },
                )?;
                totalMemoryCount += exportData.memories.len() as i32;
                totalLinkCount += exportData.links.len() as i32;
                let storeId = operit1SharedMemoryStoreId(&profileId);
                let storeName = operit1SharedMemoryStoreName(parsed, &profileId)?;
                sharedMemoryStoreManager
                    .createSharedMemoryStoreWithId(storeId.clone(), storeName)
                    .map_err(|error| format!("创建 Operit1 共享记忆库失败：{error}"))?;
                let ownerKey = sharedMemoryOwnerKey(&storeId)?;
                let repository = MemoryRepository::new(ownerKey);
                let json = serde_json::to_string(&exportData).map_err(|error| error.to_string())?;
                repository
                    .importMemoriesFromJson(json, ImportStrategy::UPDATE)
                    .map_err(|error| format!("导入 Operit1 记忆库失败：{error}"))?;
                publishOperit1SnapshotCountedProgress(
                    "memory",
                    "迁移记忆库",
                    format!(
                        "已迁移 {}/{profileCount} 个记忆库，累计 {} 条记忆和 {} 条关联。",
                        profileIndex + 1,
                        totalMemoryCount,
                        totalLinkCount,
                    ),
                    0.78,
                    0.90,
                    profileIndex + 1,
                    profileCount,
                );
            }
            if profileCount == 0 {
                publishOperit1SnapshotCountedProgress(
                    "memory",
                    "迁移记忆库",
                    "快照没有可迁移的记忆库。".to_string(),
                    0.78,
                    0.90,
                    0,
                    0,
                );
            }
            Ok((totalMemoryCount, totalLinkCount))
        })();
        result
    }

    /// Stages one snapshot entry for an operation and removes it after that operation completes.
    fn withStagedArchiveEntry<T>(
        &self,
        parsed: &ParsedOperit1Snapshot,
        entryName: &str,
        storagePath: &str,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        writeArchiveEntryToStorage(
            self.storageWriteHost.as_ref(),
            parsed,
            entryName,
            storagePath,
        )?;
        let result = operation();
        self.storageHost
            .delete(storagePath, false)
            .map_err(|error| error.to_string())?;
        result
    }

    /// Stages one SQLite sidecar entry when that sidecar exists in the snapshot.
    fn withStagedSqliteSidecarEntry<T>(
        &self,
        parsed: &ParsedOperit1Snapshot,
        entryName: &str,
        storagePath: &str,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        if parsed.archive.hasEntry(entryName) {
            return self.withStagedArchiveEntry(parsed, entryName, storagePath, operation);
        }
        operation()
    }

    /// Stages, normalizes, opens, and removes the Operit1 chat database for one operation.
    fn withStagedOperit1ChatDatabase<T>(
        &self,
        parsed: &ParsedOperit1Snapshot,
        operation: impl FnOnce(
            &mut dyn RuntimeSqliteConnection,
            Operit1ToOperit2ChatArchiveBridge,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        self.withStagedArchiveEntry(
            parsed,
            ENTRY_DATABASE,
            OPERIT1_SNAPSHOT_SQLITE_INSPECTION_PATH,
            || {
                self.withStagedSqliteSidecarEntry(
                    parsed,
                    ENTRY_DATABASE_WAL,
                    OPERIT1_SNAPSHOT_SQLITE_INSPECTION_WAL_PATH,
                    || {
                        self.withStagedSqliteSidecarEntry(
                            parsed,
                            ENTRY_DATABASE_SHM,
                            OPERIT1_SNAPSHOT_SQLITE_INSPECTION_SHM_PATH,
                            || {
                                let mut connection = self
                                    .sqliteHost
                                    .openSqliteDatabase(OPERIT1_SNAPSHOT_SQLITE_INSPECTION_PATH)
                                    .map_err(|error| error.to_string())?;
                                let archiveBridge = prepareOperit1RoomImport(connection.as_mut())?;
                                operation(connection.as_mut(), archiveBridge)
                            },
                        )
                    },
                )
            },
        )
    }

    /// Counts chat and message rows through the runtime owner's SQLite host.
    fn databaseCounts(&self, parsed: &ParsedOperit1Snapshot) -> Result<(i32, i32), String> {
        self.withStagedOperit1ChatDatabase(parsed, |connection, _| {
            operit1ChatDatabaseCounts(connection)
        })
    }
}

include!("operit1/Operit1ModelMigration.rs");
include!("operit1/Operit1PreferenceMigration.rs");
include!("operit1/Operit1CharacterCards.rs");
include!("operit1/Operit1ChatMigration.rs");
include!("operit1/Operit1MemoryStorage.rs");
include!("operit1/Operit1Parsing.rs");
