use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::convert::TryInto;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chrono::TimeZone;
use lmdb::{Cursor as LmdbCursor, Environment, EnvironmentFlags, Transaction};
use operit_store::PreferencesDataStore::{
    mutableStateFlow, stringPreferencesKey, MutableStateFlow, PreferencesDataStore, StateFlow,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStorageLayout::DATA_MEMORY_SHARED_DIR_PATH;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::ZipArchive;

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
use operit_model::ModelCatalog::ModelCatalog;
use operit_model::ModelConfigData::{
    ApiProviderType, ModelCapabilities, ModelCatalogKey, ModelConfigDefaults, ModelContextSpec,
    ModelProfile, ModelRequestSpec, ModelSummarySettings, ProviderProfile,
};
use operit_model::ModelParameter::{CustomParameterData, ModelParameter, ParameterCategory};
use operit_model::OperitChatArchive::{
    OperitArchivedChat, OperitArchivedMessage, OperitChatArchive, ARCHIVE_TYPE,
    CURRENT_FORMAT_VERSION,
};
use operit_model::PromptTag::{PromptTag, TagType};
use operit_model::StandardModelParameters::StandardModelParameters;
use operit_model::TtsConfig::{
    TtsConfig, TtsHttpHeader, TtsHttpResponsePipelineStep, TtsProviderType,
};
use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
use operit_store::repository::MemoryRepository::MemoryRepository;
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
const ENTRY_OBJECTBOX_DEFAULT_DATA: &str = "payload/files/objectbox/data.mdb";
const ENTRY_DATASTORE_PREFIX: &str = "payload/files/datastore/";
const ENTRY_FILES_PREFIX: &str = "payload/files/";
const ENTRY_WORKSPACE_FILES_PREFIX: &str = "payload/files/workspace/";
const ENTRY_EXTERNAL_FILES_PREFIX: &str = "payload/external_files/";
const KEY_CONFIG_LIST: &str = "config_list";
const KEY_FUNCTION_CONFIG_MAPPING: &str = "function_config_mapping";
const SQLITE_INSPECTION_TEMP_FILE: &str = "operit1_snapshot_import_inspect.sqlite";
const OBJECTBOX_IMPORT_TEMP_DIR: &str = "operit1_snapshot_objectbox_import";
const OPERIT1_DEFAULT_PROFILE_ID: &str = "default";
const OPERIT1_SHARED_MEMORY_STORE_ID_PREFIX: &str = "operit1-profile-";
const RUNTIME_IMPORTED_OPERIT1_FILES_DIR: &str = "runtime/imported/operit1/files";
const RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES_DIR: &str = "runtime/imported/operit1/external_files";
const OPERIT1_INTERNAL_FILES_PREFIX: &str = "/data/user/0/com.ai.assistance.operit/files/";
const OPERIT1_DATA_DATA_FILES_PREFIX: &str = "/data/data/com.ai.assistance.operit/files/";
const OPERIT1_EXTERNAL_DOWNLOAD_PREFIX: &str = "/storage/emulated/0/Download/Operit/";
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
                "已迁移 {} 个聊天、{} 条消息、{} 条记忆和 {} 个资源文件。",
                result.importedChats,
                result.importedMessages,
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
    operit1SnapshotImportProgressFlow().set_value(progress);
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
    rootDir: PathBuf,
}

impl Operit1SnapshotImportManager {
    /// Creates an importer rooted at the target runtime data directory.
    pub fn new(rootDir: PathBuf) -> Self {
        Self { rootDir }
    }

    #[allow(non_snake_case)]
    /// Reads an Operit1 snapshot and returns import preview metadata.
    pub fn inspectSnapshot(&self, bytes: Vec<u8>) -> Result<Operit1SnapshotPreview, String> {
        let parsed = ParsedOperit1Snapshot::fromBytes(bytes)?;
        parsed.preview()
    }

    #[allow(non_snake_case)]
    /// Imports a full Operit1 snapshot into runtime storage.
    pub fn importSnapshot(&self, bytes: Vec<u8>) -> Result<Operit1SnapshotImportResult, String> {
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress::stage(
            "parse",
            "解析快照",
            "正在读取清单、配置和数据索引。",
            0.08,
        ));
        let parsed = ParsedOperit1Snapshot::fromBytes(bytes)?;
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
        let fileImportPlan = SnapshotFileImportPlan::new(&self.rootDir);
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
        let fileImportResult = self.importSnapshotFiles(&parsed, &fileImportPlan)?;
        let result = Operit1SnapshotImportResult {
            modelConfig,
            importedDatastoreFiles,
            importedDatastoreKeys,
            importedChats,
            importedMessages,
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
        let paths = RuntimeStorePaths::new(self.rootDir.clone());
        let promptTags = buildOperit2PromptTags(parsed)?;
        if !promptTags.is_empty()
            || parsed
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
            appendSharedUserMarkdown(&self.rootDir, &profileId, &markdown)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Reads only model configuration preview metadata from an Operit1 snapshot.
    pub fn inspectModelConfigSnapshot(
        &self,
        bytes: Vec<u8>,
    ) -> Result<Operit1ModelConfigSnapshotPreview, String> {
        let parsed = ParsedOperit1Snapshot::fromBytes(bytes)?;
        parsed.modelConfigPreview()
    }

    #[allow(non_snake_case)]
    /// Imports one selected Operit1 model configuration and model binding.
    pub fn importModelConfigSnapshot(
        &self,
        bytes: Vec<u8>,
        configId: String,
        modelId: String,
    ) -> Result<Operit1ModelConfigImportResult, String> {
        let parsed = ParsedOperit1Snapshot::fromBytes(bytes)?;
        self.importModelConfigFromParsed(&parsed, configId, modelId)
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
        let providerType = config.providerType()?;
        let providerCatalog = ModelCatalog::provider(providerType.name())?;
        let endpoint = config.apiEndpoint.trim().to_string();
        if endpoint.is_empty() {
            return Err(format!("模型配置「{}」缺少服务地址", config.name));
        }

        let mut provider = ProviderProfile::new(
            ModelConfigDefaults::DEFAULT_PROVIDER_ID.to_string(),
            providerCatalog.displayName,
            providerType,
            endpoint,
        );
        provider.providerTypeId = provider.providerType.name().to_string();
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
            .map(|currentModelId| {
                buildModelProfile(&provider.providerTypeId, currentModelId, &config)
            })
            .collect::<Result<Vec<_>, String>>()?;

        let modelConfigManager = ModelConfigManager::new(self.rootDir.clone());
        modelConfigManager
            .replaceDefaultProviderProfile(provider.clone())
            .map_err(|error| error.to_string())?;

        let functionalConfigManager = FunctionalConfigManager::new(self.rootDir.clone());
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
    fn importDataStorePreferences(
        &self,
        parsed: &ParsedOperit1Snapshot,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<(i32, i32), String> {
        let paths = RuntimeStorePaths::new(self.rootDir.clone());
        let mappings = datastorePreferenceMappings(&paths);
        let mut fileCount = 0;
        let mut keyCount = 0;
        for (entryName, filePath) in mappings {
            let Some(preferences) = parsed.datastorePreferences.get(&entryName) else {
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
        }
        Ok((fileCount, keyCount))
    }

    #[allow(non_snake_case)]
    fn importChatDatabase(
        &self,
        parsed: &ParsedOperit1Snapshot,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<(i32, i32), String> {
        let databaseBytes = parsed
            .entries
            .get(ENTRY_DATABASE)
            .ok_or_else(|| format!("Operit1 快照缺少聊天数据库：{ENTRY_DATABASE}"))?;
        let tempPath = self
            .rootDir
            .join("runtime")
            .join("temp")
            .join(SQLITE_INSPECTION_TEMP_FILE);
        writeTempFile(&tempPath, databaseBytes)?;
        let result = (|| {
            let archive = buildChatArchiveFromOperit1Database(&tempPath, fileImportPlan)?;
            let chatCount = archive.chats.len() as i32;
            let messageCount = archive
                .chats
                .iter()
                .map(|chat| chat.messages.len() as i32)
                .sum();
            let json = serde_json::to_string(&archive).map_err(|error| error.to_string())?;
            let manager = ChatHistoryManager::new(RuntimeStorePaths::new(self.rootDir.clone()))
                .map_err(|error| error.to_string())?;
            manager
                .importChatHistoriesFromJson(json)
                .map_err(|error| error.to_string())?;
            Ok((chatCount, messageCount))
        })();
        let _ = fs::remove_file(&tempPath);
        result
    }

    #[allow(non_snake_case)]
    fn importSnapshotFiles(
        &self,
        parsed: &ParsedOperit1Snapshot,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<Operit1SnapshotFileImportResult, String> {
        let (importedWorkspaces, importedWorkspaceFiles) =
            copyWorkspaceEntries(&parsed.entries, fileImportPlan)?;
        let importedFiles = copyEntriesWithPrefix(
            &self.rootDir,
            &parsed.entries,
            ENTRY_FILES_PREFIX,
            RUNTIME_IMPORTED_OPERIT1_FILES_DIR,
        )?;
        let importedExternalFiles = copyEntriesWithPrefix(
            &self.rootDir,
            &parsed.entries,
            ENTRY_EXTERNAL_FILES_PREFIX,
            RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES_DIR,
        )?;
        Ok(Operit1SnapshotFileImportResult {
            importedFiles,
            importedExternalFiles,
            importedWorkspaces,
            importedWorkspaceFiles,
        })
    }

    #[allow(non_snake_case)]
    fn importObjectBoxMemoryStore(
        &self,
        parsed: &ParsedOperit1Snapshot,
    ) -> Result<(i32, i32), String> {
        let tempDir = self
            .rootDir
            .join("runtime")
            .join("temp")
            .join(OBJECTBOX_IMPORT_TEMP_DIR);
        if tempDir.exists() {
            fs::remove_dir_all(&tempDir).map_err(|error| error.to_string())?;
        }
        let result = (|| {
            let paths = RuntimeStorePaths::new(self.rootDir.clone());
            let sharedMemoryStoreManager = SharedMemoryStoreManager::new(paths);
            let profiles = collectOperit1MemoryProfileIds(parsed)?;
            let mut totalMemoryCount = 0;
            let mut totalLinkCount = 0;
            for profileId in profiles {
                let objectBoxBytes = operit1ObjectBoxDataForProfile(parsed, &profileId)?;
                if tempDir.exists() {
                    fs::remove_dir_all(&tempDir).map_err(|error| error.to_string())?;
                }
                fs::create_dir_all(&tempDir).map_err(|error| error.to_string())?;
                writeFile(&tempDir.join("data.mdb"), objectBoxBytes)?;
                let exportData = buildMemoryExportDataFromOperit1ObjectBox(&tempDir)?;
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
            }
            Ok((totalMemoryCount, totalLinkCount))
        })();
        if tempDir.exists() {
            fs::remove_dir_all(&tempDir).map_err(|error| error.to_string())?;
        }
        result
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1Manifest {
    formatVersion: i32,
    packageName: String,
    createdAt: i64,
}

struct ParsedOperit1Snapshot {
    manifest: Operit1Manifest,
    entries: BTreeMap<String, Vec<u8>>,
    datastorePreferences: BTreeMap<String, HashMap<String, Operit1PreferenceValue>>,
    configs: Vec<Operit1ModelConfig>,
    chatMapping: Operit1FunctionConfigMapping,
}

impl ParsedOperit1Snapshot {
    #[allow(non_snake_case)]
    fn fromBytes(bytes: Vec<u8>) -> Result<Self, String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
        let entries = readZipEntries(&mut archive)?;
        let manifestText = String::from_utf8(
            entries
                .get(ENTRY_MANIFEST)
                .ok_or_else(|| "Operit1 快照缺少 manifest.json".to_string())?
                .clone(),
        )
        .map_err(|error| error.to_string())?;
        let manifest: Operit1Manifest = serde_json::from_str(&manifestText)
            .map_err(|error| format!("Operit1 快照清单格式不正确：{error}"))?;
        if manifest.formatVersion != FORMAT_VERSION {
            return Err(format!(
                "暂不支持这个 Operit1 快照版本：{}",
                manifest.formatVersion
            ));
        }

        let datastorePreferences = decodeAllDataStorePreferences(&entries)?;
        let modelPrefs = datastorePreferences
            .get(ENTRY_MODEL_CONFIGS)
            .ok_or_else(|| "快照里没有 Operit1 模型配置文件".to_string())?;
        let configListJson = requiredPreferenceString(
            modelPrefs,
            KEY_CONFIG_LIST,
            "快照里没有 Operit1 模型配置列表",
        )?;
        let configIds: Vec<String> = serde_json::from_str(configListJson)
            .map_err(|error| format!("模型配置列表格式不正确：{error}"))?;
        if configIds.is_empty() {
            return Err("快照里的模型配置列表为空".to_string());
        }

        let mut configs = Vec::new();
        for configId in configIds {
            let key = format!("config_{configId}");
            let configJson = requiredPreferenceString(
                modelPrefs,
                &key,
                &format!("快照缺少模型配置：{configId}"),
            )?;
            configs.push(
                serde_json::from_str(configJson)
                    .map_err(|error| format!("模型配置「{configId}」格式不正确：{error}"))?,
            );
        }

        let functionalPrefs = datastorePreferences
            .get(ENTRY_FUNCTIONAL_CONFIGS)
            .ok_or_else(|| "快照里没有 Operit1 功能配置文件".to_string())?;
        let chatMappingJson = requiredPreferenceString(
            functionalPrefs,
            KEY_FUNCTION_CONFIG_MAPPING,
            "快照里没有 Operit1 功能模型映射",
        )?;
        let chatMapping = decodeChatMapping(chatMappingJson)?;

        Ok(Self {
            manifest,
            entries,
            datastorePreferences,
            configs,
            chatMapping,
        })
    }

    fn preview(&self) -> Result<Operit1SnapshotPreview, String> {
        let modelConfig = self.modelConfigPreview()?;
        let datastoreFiles = self
            .datastorePreferences
            .iter()
            .map(|(entryName, values)| Operit1DataStoreFilePreview {
                fileName: entryName
                    .strip_prefix(ENTRY_DATASTORE_PREFIX)
                    .unwrap_or(entryName)
                    .to_string(),
                keyCount: values.len() as i32,
            })
            .collect::<Vec<_>>();
        let (chatCount, messageCount) = self.databaseCounts()?;
        let importedFileCount = self.countImportableFiles(ENTRY_FILES_PREFIX);
        let importedExternalFileCount = self.countImportableFiles(ENTRY_EXTERNAL_FILES_PREFIX);
        let mut detectedDomains = Vec::new();
        if !self.configs.is_empty() {
            detectedDomains.push("model_configs".to_string());
        }
        if chatCount > 0 || messageCount > 0 {
            detectedDomains.push("chat_history".to_string());
        }
        for entry in self.datastorePreferences.keys() {
            let name = entry
                .strip_prefix(ENTRY_DATASTORE_PREFIX)
                .unwrap_or(entry)
                .trim_end_matches(".preferences_pb")
                .to_string();
            if !detectedDomains.contains(&name) {
                detectedDomains.push(name);
            }
        }
        if importedFileCount > 0 || importedExternalFileCount > 0 {
            detectedDomains.push("user_files".to_string());
        }
        Ok(Operit1SnapshotPreview {
            formatVersion: self.manifest.formatVersion,
            packageName: self.manifest.packageName.clone(),
            createdAt: self.manifest.createdAt,
            modelConfig,
            datastoreFiles,
            chatCount,
            messageCount,
            importedFileCount,
            importedExternalFileCount,
            detectedDomains,
        })
    }

    #[allow(non_snake_case)]
    fn modelConfigPreview(&self) -> Result<Operit1ModelConfigSnapshotPreview, String> {
        let configs = self
            .configs
            .iter()
            .map(|config| {
                let providerType = config.providerType()?;
                let provider = ModelCatalog::provider(providerType.name())?;
                let modelIds = splitModelIds(&config.modelName);
                let selectedModelIndex =
                    (self.chatMapping.configId == config.id).then_some(self.chatMapping.modelIndex);
                let selectedModelId =
                    selectedModelIndex.and_then(|index| modelIds.get(index as usize).cloned());
                Ok(Operit1ModelConfigPreview {
                    configId: config.id.clone(),
                    name: config.name.clone(),
                    providerTypeId: providerType.name().to_string(),
                    providerDisplayName: provider.displayName,
                    endpoint: config.apiEndpoint.clone(),
                    modelIds,
                    selectedModelId,
                    selectedModelIndex,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        let chatConfigId = Some(self.chatMapping.configId.clone());
        let chatModelIndex = Some(self.chatMapping.modelIndex);
        let chatModelId = self
            .configs
            .iter()
            .find(|config| config.id == self.chatMapping.configId)
            .and_then(|config| {
                splitModelIds(&config.modelName)
                    .get(self.chatMapping.modelIndex as usize)
                    .cloned()
            });

        Ok(Operit1ModelConfigSnapshotPreview {
            formatVersion: self.manifest.formatVersion,
            packageName: self.manifest.packageName.clone(),
            createdAt: self.manifest.createdAt,
            configs,
            chatConfigId,
            chatModelId,
            chatModelIndex,
        })
    }

    #[allow(non_snake_case)]
    fn selectedChatConfig(&self) -> Result<Operit1ModelConfigPreview, String> {
        self.modelConfigPreview()?
            .configs
            .into_iter()
            .find(|config| config.configId == self.chatMapping.configId)
            .ok_or_else(|| format!("快照里没有聊天模型配置：{}", self.chatMapping.configId))
    }

    #[allow(non_snake_case)]
    fn configById(&self, configId: &str) -> Result<Operit1ModelConfig, String> {
        self.configs
            .iter()
            .find(|config| config.id == configId)
            .cloned()
            .ok_or_else(|| format!("快照里没有模型配置：{configId}"))
    }

    #[allow(non_snake_case)]
    /// Counts chat and message rows in the snapshot database.
    fn databaseCounts(&self) -> Result<(i32, i32), String> {
        let databaseBytes = self
            .entries
            .get(ENTRY_DATABASE)
            .ok_or_else(|| format!("Operit1 快照缺少聊天数据库：{ENTRY_DATABASE}"))?;
        let tempPath = std::env::temp_dir().join(SQLITE_INSPECTION_TEMP_FILE);
        writeTempFile(&tempPath, databaseBytes)?;
        let result = (|| {
            let connection = Connection::open(&tempPath).map_err(|error| error.to_string())?;
            if !sqliteTableExists(&connection, "chats")?
                || !sqliteTableExists(&connection, "messages")?
            {
                return Ok((0, 0));
            }
            let chatCount = queryCount(&connection, "SELECT COUNT(*) FROM chats")?;
            let messageCount = queryCount(&connection, "SELECT COUNT(*) FROM messages")?;
            Ok((chatCount, messageCount))
        })();
        let _ = fs::remove_file(&tempPath);
        result
    }

    #[allow(non_snake_case)]
    fn countImportableFiles(&self, prefix: &str) -> i32 {
        self.entries
            .keys()
            .filter(|entry| entry.starts_with(prefix))
            .filter(|entry| !isDataStoreEntry(entry))
            .count() as i32
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1FunctionConfigMapping {
    #[serde(default = "defaultOperit1ConfigId")]
    configId: String,
    #[serde(default)]
    modelIndex: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1ModelConfig {
    id: String,
    name: String,
    #[serde(default)]
    apiKey: String,
    #[serde(default)]
    apiEndpoint: String,
    #[serde(default)]
    modelName: String,
    #[serde(default)]
    apiProviderType: String,
    #[serde(default)]
    apiProviderTypeId: String,
    #[serde(default)]
    useMultipleApiKeys: bool,
    #[serde(default)]
    apiKeyPool: Vec<Operit1ApiKeyInfo>,
    #[serde(default)]
    currentKeyIndex: i32,
    #[serde(default = "defaultKeyRotationMode")]
    keyRotationMode: String,
    #[serde(default)]
    hasCustomParameters: bool,
    #[serde(default)]
    maxTokensEnabled: bool,
    #[serde(default)]
    temperatureEnabled: bool,
    #[serde(default)]
    topPEnabled: bool,
    #[serde(default)]
    topKEnabled: bool,
    #[serde(default)]
    presencePenaltyEnabled: bool,
    #[serde(default)]
    frequencyPenaltyEnabled: bool,
    #[serde(default)]
    repetitionPenaltyEnabled: bool,
    #[serde(default = "defaultMaxTokens")]
    maxTokens: i32,
    #[serde(default = "defaultTemperature")]
    temperature: f32,
    #[serde(default = "defaultTopP")]
    topP: f32,
    #[serde(default)]
    topK: i32,
    #[serde(default)]
    presencePenalty: f32,
    #[serde(default)]
    frequencyPenalty: f32,
    #[serde(default = "defaultRepetitionPenalty")]
    repetitionPenalty: f32,
    #[serde(default = "defaultCustomParameters")]
    customParameters: String,
    #[serde(default = "defaultCustomHeaders")]
    customHeaders: String,
    #[serde(default = "defaultMaxContextLength")]
    maxContextLength: f32,
    #[serde(default)]
    enableMaxContextMode: bool,
    #[serde(default = "defaultSummaryTokenThreshold")]
    summaryTokenThreshold: f32,
    #[serde(default = "defaultEnableSummary")]
    enableSummary: bool,
    #[serde(default = "defaultEnableSummaryByMessageCount")]
    enableSummaryByMessageCount: bool,
    #[serde(default = "defaultSummaryMessageCountThreshold")]
    summaryMessageCountThreshold: i32,
    #[serde(default)]
    mnnForwardType: i32,
    #[serde(default = "defaultThreadCount")]
    mnnThreadCount: i32,
    #[serde(default = "defaultThreadCount")]
    llamaThreadCount: i32,
    #[serde(default = "defaultLlamaContextSize")]
    llamaContextSize: i32,
    #[serde(default = "defaultLlamaBatchSize")]
    llamaBatchSize: i32,
    #[serde(default = "defaultLlamaBatchSize")]
    llamaUBatchSize: i32,
    #[serde(default)]
    llamaGpuLayers: i32,
    #[serde(default)]
    llamaUseMmap: bool,
    #[serde(default)]
    llamaFlashAttention: bool,
    #[serde(default = "defaultLlamaKvUnified")]
    llamaKvUnified: bool,
    #[serde(default)]
    llamaOffloadKqv: bool,
    #[serde(default)]
    enableDirectImageProcessing: bool,
    #[serde(default)]
    enableDirectAudioProcessing: bool,
    #[serde(default)]
    enableDirectVideoProcessing: bool,
    #[serde(default)]
    enableToolCall: bool,
    #[serde(default)]
    requestLimitPerMinute: i32,
    #[serde(default)]
    maxConcurrentRequests: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1ApiKeyInfo {
    id: String,
    key: String,
    #[serde(default)]
    name: String,
    #[serde(default = "defaultTrue")]
    isEnabled: bool,
    #[serde(default = "defaultApiKeyAvailabilityStatus")]
    availabilityStatus: ApiKeyAvailabilityStatus,
    #[serde(default)]
    usageCount: i64,
    #[serde(default)]
    lastUsed: i64,
    #[serde(default)]
    errorCount: i64,
}

impl Operit1ApiKeyInfo {
    #[allow(non_snake_case)]
    fn toApiKeyInfo(&self) -> ApiKeyInfo {
        ApiKeyInfo {
            id: self.id.clone(),
            key: self.key.clone(),
            name: self.name.clone(),
            isEnabled: self.isEnabled,
            availabilityStatus: self.availabilityStatus.clone(),
            usageCount: self.usageCount,
            lastUsed: self.lastUsed,
            errorCount: self.errorCount,
        }
    }
}

impl Operit1ModelConfig {
    #[allow(non_snake_case)]
    fn providerType(&self) -> Result<ApiProviderType, String> {
        let providerTypeId = if !self.apiProviderTypeId.trim().is_empty() {
            self.apiProviderTypeId.trim()
        } else if !self.apiProviderType.trim().is_empty() {
            self.apiProviderType.trim()
        } else {
            ApiProviderType::DEEPSEEK.name()
        };
        ApiProviderType::fromProviderTypeId(providerTypeId)
            .ok_or_else(|| format!("无法识别 Operit1 供应商类型：{}", providerTypeId))
    }
}

#[allow(non_snake_case)]
fn buildModelProfile(
    providerTypeId: &str,
    modelId: &str,
    config: &Operit1ModelConfig,
) -> Result<ModelProfile, String> {
    let mut model = ModelProfile::new(modelId.to_string());
    if ModelCatalog::model(providerTypeId, modelId).is_ok() {
        model.catalogKey = Some(ModelCatalogKey {
            providerTypeId: providerTypeId.to_string(),
            modelId: modelId.to_string(),
        });
    }
    model.contextOverride = Some(ModelContextSpec {
        maxContextLength: config.maxContextLength,
        enableMaxContextMode: config.enableMaxContextMode,
    });
    model.capabilitiesOverride = Some(ModelCapabilities {
        directImage: config.enableDirectImageProcessing,
        directAudio: config.enableDirectAudioProcessing,
        directVideo: config.enableDirectVideoProcessing,
        toolCall: config.enableToolCall,
    });
    model.requestOverride = Some(ModelRequestSpec {
        supportsStructuredTools: config.enableToolCall,
    });
    model.summary = ModelSummarySettings {
        enableSummary: config.enableSummary,
        summaryTokenThreshold: config.summaryTokenThreshold,
        enableSummaryByMessageCount: config.enableSummaryByMessageCount,
        summaryMessageCountThreshold: config.summaryMessageCountThreshold,
    };
    model.localRuntime.mnnForwardType = config.mnnForwardType;
    model.localRuntime.mnnThreadCount = config.mnnThreadCount;
    model.localRuntime.llamaThreadCount = config.llamaThreadCount;
    model.localRuntime.llamaContextSize = config.llamaContextSize;
    model.localRuntime.llamaBatchSize = config.llamaBatchSize;
    model.localRuntime.llamaUBatchSize = config.llamaUBatchSize;
    model.localRuntime.llamaGpuLayers = config.llamaGpuLayers;
    model.localRuntime.llamaUseMmap = config.llamaUseMmap;
    model.localRuntime.llamaFlashAttention = config.llamaFlashAttention;
    model.localRuntime.llamaKvUnified = config.llamaKvUnified;
    model.localRuntime.llamaOffloadKqv = config.llamaOffloadKqv;
    model.parameters = buildModelParameters(config)?;
    Ok(model)
}

#[allow(non_snake_case)]
fn buildModelParameters(config: &Operit1ModelConfig) -> Result<Vec<ModelParameter<Value>>, String> {
    let mut parameters = Vec::new();
    pushStandardParameter(
        &mut parameters,
        "max_tokens",
        config.maxTokensEnabled,
        serde_json::json!(config.maxTokens),
    )?;
    pushStandardParameter(
        &mut parameters,
        "temperature",
        config.temperatureEnabled,
        serde_json::json!(config.temperature),
    )?;
    pushStandardParameter(
        &mut parameters,
        "top_p",
        config.topPEnabled,
        serde_json::json!(config.topP),
    )?;
    pushStandardParameter(
        &mut parameters,
        "top_k",
        config.topKEnabled,
        serde_json::json!(config.topK),
    )?;
    pushStandardParameter(
        &mut parameters,
        "presence_penalty",
        config.presencePenaltyEnabled,
        serde_json::json!(config.presencePenalty),
    )?;
    pushStandardParameter(
        &mut parameters,
        "frequency_penalty",
        config.frequencyPenaltyEnabled,
        serde_json::json!(config.frequencyPenalty),
    )?;
    pushStandardParameter(
        &mut parameters,
        "repetition_penalty",
        config.repetitionPenaltyEnabled,
        serde_json::json!(config.repetitionPenalty),
    )?;
    if config.hasCustomParameters && config.customParameters.trim() != "[]" {
        let customParameters: Vec<CustomParameterData> =
            serde_json::from_str(&config.customParameters)
                .map_err(|error| format!("Operit1 自定义模型参数格式不正确：{error}"))?;
        for parameter in customParameters {
            parameters.push(ModelParameter {
                id: parameter.id,
                name: parameter.name,
                apiName: parameter.apiName,
                description: parameter.description,
                defaultValue: parseCustomParameterValue(&parameter.defaultValue)?,
                currentValue: parseCustomParameterValue(&parameter.currentValue)?,
                isEnabled: parameter.isEnabled,
                valueType: parseParameterValueType(&parameter.valueType)?,
                minValue: parameter
                    .minValue
                    .map(|value| parseCustomParameterValue(&value))
                    .transpose()?,
                maxValue: parameter
                    .maxValue
                    .map(|value| parseCustomParameterValue(&value))
                    .transpose()?,
                category: parseParameterCategory(&parameter.category)?,
                isCustom: true,
            });
        }
    }
    Ok(parameters)
}

#[allow(non_snake_case)]
fn pushStandardParameter(
    parameters: &mut Vec<ModelParameter<Value>>,
    id: &str,
    enabled: bool,
    value: Value,
) -> Result<(), String> {
    let definition = StandardModelParameters::DEFINITIONS()
        .into_iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| format!("标准模型参数不存在：{id}"))?;
    parameters.push(ModelParameter {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        apiName: definition.apiName.to_string(),
        description: definition.description.to_string(),
        defaultValue: definition.defaultValue,
        currentValue: value,
        isEnabled: enabled,
        valueType: definition.valueType,
        minValue: definition.minValue,
        maxValue: definition.maxValue,
        category: definition.category,
        isCustom: false,
    });
    Ok(())
}

#[allow(non_snake_case)]
fn decodeChatMapping(json: &str) -> Result<Operit1FunctionConfigMapping, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|error| format!("功能模型映射格式不正确：{error}"))?;
    let chat = value
        .get("CHAT")
        .ok_or_else(|| "功能模型映射缺少 CHAT".to_string())?;
    if let Some(configId) = chat.as_str() {
        return Ok(Operit1FunctionConfigMapping {
            configId: configId.to_string(),
            modelIndex: 0,
        });
    }
    serde_json::from_value(chat.clone())
        .map_err(|error| format!("CHAT 模型映射格式不正确：{error}"))
}

#[allow(non_snake_case)]
fn splitModelIds(modelName: &str) -> Vec<String> {
    modelName
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[allow(non_snake_case)]
fn readZipText(archive: &mut ZipArchive<Cursor<Vec<u8>>>, name: &str) -> Result<String, String> {
    let bytes = readZipBytes(archive, name)?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn readZipBytes(archive: &mut ZipArchive<Cursor<Vec<u8>>>, name: &str) -> Result<Vec<u8>, String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| format!("快照缺少必要文件：{name}"))?;
    if file.is_dir() {
        return Err(format!("快照条目不是文件：{name}"));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

#[allow(non_snake_case)]
fn readZipEntries(
    archive: &mut ZipArchive<Cursor<Vec<u8>>>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        validateSnapshotEntryPath(&name)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        entries.insert(name, bytes);
    }
    Ok(entries)
}

#[allow(non_snake_case)]
fn decodeAllDataStorePreferences(
    entries: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, HashMap<String, Operit1PreferenceValue>>, String> {
    let mut result = BTreeMap::new();
    for (entry, bytes) in entries {
        if isDataStoreEntry(entry) {
            result.insert(entry.clone(), decodeDataStorePreferences(bytes)?);
        }
    }
    Ok(result)
}

#[derive(Clone, Debug, PartialEq)]
enum Operit1PreferenceValue {
    Boolean(bool),
    Float(f32),
    Double(f64),
    Int(i32),
    String(String),
    StringSet(Vec<String>),
    Long(i64),
}

impl Operit1PreferenceValue {
    #[allow(non_snake_case)]
    fn asString(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    #[allow(non_snake_case)]
    fn asStringSet(&self) -> Option<&[String]> {
        match self {
            Self::StringSet(value) => Some(value),
            _ => None,
        }
    }

    #[allow(non_snake_case)]
    fn toTargetPreferenceStringForKey(
        &self,
        key: &str,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<String, String> {
        match self {
            Self::Boolean(value) => Ok(value.to_string()),
            Self::Float(value) => Ok(value.to_string()),
            Self::Double(value) => Ok(value.to_string()),
            Self::Int(value) => Ok(value.to_string()),
            Self::String(value) => fileImportPlan.rewritePreferenceValue(key, value),
            Self::StringSet(value) => {
                serde_json::to_string(value).map_err(|error| error.to_string())
            }
            Self::Long(value) => Ok(value.to_string()),
        }
    }

    #[allow(non_snake_case)]
    fn toTargetPreferenceEntry(
        &self,
        key: &str,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<(String, String), String> {
        let targetKey = fileImportPlan.rewritePreferenceKey(key)?;
        let targetValue = self.toTargetPreferenceStringForKey(key, fileImportPlan)?;
        Ok((targetKey, targetValue))
    }
}

#[derive(Clone, Debug)]
struct SnapshotFileImportPlan {
    rootDir: PathBuf,
    importedFilesRoot: PathBuf,
    importedExternalFilesRoot: PathBuf,
    workspaceRoot: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Operit1SnapshotFileImportResult {
    importedFiles: i32,
    importedExternalFiles: i32,
    importedWorkspaces: i32,
    importedWorkspaceFiles: i32,
}

impl SnapshotFileImportPlan {
    fn new(rootDir: &Path) -> Self {
        let paths = RuntimeStorePaths::new(rootDir.to_path_buf());
        Self {
            rootDir: rootDir.to_path_buf(),
            importedFilesRoot: rootDir.join(RUNTIME_IMPORTED_OPERIT1_FILES_DIR),
            importedExternalFilesRoot: rootDir.join(RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES_DIR),
            workspaceRoot: paths.workspace_dir(),
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
    fn rewriteChatWorkspace(&self, workspace: Option<String>) -> Result<Option<String>, String> {
        let Some(workspace) = workspace else {
            return Ok(None);
        };
        self.rewriteWorkspacePath(&workspace).map(Some)
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
        Ok(self
            .importedFilesRoot
            .join(relative)
            .to_string_lossy()
            .replace('\\', "/"))
    }

    #[allow(non_snake_case)]
    fn rewriteExternalDownloadRelativePath(&self, relative: &str) -> Result<String, String> {
        validateRelativePath(relative)?;
        if let Some(workspaceRelative) = relative.strip_prefix("workspace/") {
            let (workspaceId, rest) = splitWorkspaceRelativePath(workspaceRelative)?;
            return Ok(workspaceVfsPath(workspaceId, rest));
        }
        Ok(self
            .importedExternalFilesRoot
            .join(relative)
            .to_string_lossy()
            .replace('\\', "/"))
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
        paths
            .root_dir()
            .join("runtime/config/preferences/user_preferences.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/api_settings.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/api_settings.json"),
    );
    mappings.insert(
        "payload/files/datastore/display_preferences.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/display_preferences.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/ui_preferences.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/ui_preferences.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/waifu_settings.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/waifu_settings.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/wake_word_preferences.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/wake_word_preferences.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/custom_emoji_settings.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/custom_emoji_settings.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/android_permission_preferences.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/android_permission_preferences.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/database_backup_settings.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/database_backup_settings.preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/github_auth_preferences.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/github_auth_preferences.json"),
    );
    mappings.insert(
        "payload/files/datastore/persona_card_chat_history.preferences_pb".to_string(),
        paths
            .root_dir()
            .join("runtime/config/preferences/persona_card_chat_history.preferences.json"),
    );
    mappings
}

#[allow(non_snake_case)]
fn buildOperit2PromptTags(parsed: &ParsedOperit1Snapshot) -> Result<Vec<PromptTag>, String> {
    let Some(preferences) = parsed.datastorePreferences.get(ENTRY_PROMPT_TAGS) else {
        return Ok(Vec::new());
    };
    let ids = optionalPreferenceStringSet(preferences, "prompt_tag_list")?;
    let legacyTagIds = collectOperit1LegacyPromptTagIds(preferences)?;
    let mut tags = Vec::new();
    for id in ids {
        if legacyTagIds.contains(&id) {
            continue;
        }
        let name = requiredPreferenceString(
            preferences,
            &format!("prompt_tag_{id}_name"),
            &format!("Operit1 提示标签缺少名称：{id}"),
        )?;
        tags.push(PromptTag {
            id: id.clone(),
            name: name.to_string(),
            description: optionalPreferenceString(
                preferences,
                &format!("prompt_tag_{id}_description"),
            )?
            .unwrap_or_default(),
            promptContent: optionalPreferenceString(
                preferences,
                &format!("prompt_tag_{id}_prompt_content"),
            )?
            .unwrap_or_default(),
            tagType: parseOperit1PromptTagType(
                optionalPreferenceString(preferences, &format!("prompt_tag_{id}_tag_type"))?
                    .as_deref(),
            )?,
            createdAt: optionalPreferenceI64(preferences, &format!("prompt_tag_{id}_created_at"))?
                .unwrap_or_else(currentTimeMillis),
            updatedAt: optionalPreferenceI64(preferences, &format!("prompt_tag_{id}_updated_at"))?
                .unwrap_or_else(currentTimeMillis),
        });
    }
    Ok(tags)
}

#[allow(non_snake_case)]
fn buildOperit2CharacterCards(
    parsed: &ParsedOperit1Snapshot,
    fileImportPlan: &SnapshotFileImportPlan,
) -> Result<Vec<CharacterCard>, String> {
    let Some(preferences) = parsed.datastorePreferences.get(ENTRY_CHARACTER_CARDS) else {
        return Ok(Vec::new());
    };
    let legacyPromptTagIds = parsed
        .datastorePreferences
        .get(ENTRY_PROMPT_TAGS)
        .map(collectOperit1LegacyPromptTagIds)
        .transpose()?
        .unwrap_or_default();
    let ids = optionalPreferenceStringSet(preferences, "character_card_list")?;
    let mut cards = Vec::new();
    for id in ids {
        let name = requiredPreferenceString(
            preferences,
            &format!("character_card_{id}_name"),
            &format!("Operit1 角色卡缺少名称：{id}"),
        )?;
        let chatModelBindingMode = optionalPreferenceString(
            preferences,
            &format!("character_card_{id}_chat_model_binding_mode"),
        )?;
        let chatModelId = resolveOperit1CharacterChatModelId(parsed, preferences, &id)?;
        let memoryBinding = resolveOperit1CharacterMemoryBinding(parsed, preferences, &id)?;
        cards.push(CharacterCard {
            id: id.clone(),
            name: name.to_string(),
            description: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_description"),
            )?
            .unwrap_or_default(),
            characterSetting: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_character_setting"),
            )?
            .unwrap_or_default(),
            openingStatement: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_opening_statement"),
            )?
            .unwrap_or_default(),
            otherContentChat: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_other_content_chat"),
            )?
            .unwrap_or_default(),
            otherContentVoice: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_other_content_voice"),
            )?
            .unwrap_or_default(),
            avatarUri: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_avatar_uri"),
            )?
            .map(|value| fileImportPlan.rewritePath(&value))
            .transpose()?
            .filter(|value| !value.trim().is_empty()),
            attachedTagIds: optionalPreferenceStringSet(
                preferences,
                &format!("character_card_{id}_attached_tag_ids"),
            )?
            .into_iter()
            .filter(|tagId| !legacyPromptTagIds.contains(tagId))
            .collect(),
            advancedCustomPrompt: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_advanced_custom_prompt"),
            )?
            .unwrap_or_default(),
            marks: optionalPreferenceString(preferences, &format!("character_card_{id}_marks"))?
                .unwrap_or_default(),
            chatModelBindingMode: if chatModelBindingMode.as_deref() == Some("FIXED_CONFIG") {
                CharacterCardChatModelBindingMode::FIXED_MODEL.to_string()
            } else {
                CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string()
            },
            chatModelId,
            ttsConfigId: None,
            memoryBindingMode: memoryBinding.memoryBindingMode,
            sharedMemoryId: memoryBinding.sharedMemoryId,
            sharedMemoryMounts: Vec::new(),
            toolAccessConfig: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_tool_access_config_json"),
            )?
            .map(|raw| serde_json::from_str::<CharacterCardToolAccessConfig>(&raw))
            .transpose()
            .map_err(|error| format!("Operit1 角色卡工具权限格式不正确：{id}: {error}"))?
            .unwrap_or_default(),
            isDefault: optionalPreferenceBool(
                preferences,
                &format!("character_card_{id}_is_default"),
            )?
            .unwrap_or(id == CharacterCardManager::DEFAULT_CHARACTER_CARD_ID),
            createdAt: optionalPreferenceI64(
                preferences,
                &format!("character_card_{id}_created_at"),
            )?
            .unwrap_or_else(currentTimeMillis),
            updatedAt: optionalPreferenceI64(
                preferences,
                &format!("character_card_{id}_updated_at"),
            )?
            .unwrap_or_else(currentTimeMillis),
        });
    }
    Ok(cards)
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1CharacterMemoryBinding {
    memoryBindingMode: String,
    sharedMemoryId: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1UserPreferenceProfile {
    id: String,
    name: String,
    #[serde(default)]
    birthDate: i64,
    #[serde(default)]
    gender: String,
    #[serde(default)]
    personality: String,
    #[serde(default)]
    identity: String,
    #[serde(default)]
    occupation: String,
    #[serde(default)]
    aiStyle: String,
    #[serde(default)]
    isInitialized: bool,
}

#[allow(non_snake_case)]
fn resolveOperit1CharacterMemoryBinding(
    parsed: &ParsedOperit1Snapshot,
    preferences: &HashMap<String, Operit1PreferenceValue>,
    cardId: &str,
) -> Result<Operit1CharacterMemoryBinding, String> {
    let bindingMode = optionalPreferenceString(
        preferences,
        &format!("character_card_{cardId}_memory_profile_binding_mode"),
    )?;
    if bindingMode.as_deref() == Some("FIXED_PROFILE") {
        let profileId = requiredPreferenceString(
            preferences,
            &format!("character_card_{cardId}_memory_profile_id"),
            &format!("Operit1 角色卡固定记忆库缺少配置 ID：{cardId}"),
        )?;
        return Ok(Operit1CharacterMemoryBinding {
            memoryBindingMode: CharacterCardMemoryBindingMode::SHARED.to_string(),
            sharedMemoryId: Some(operit1SharedMemoryStoreId(profileId)),
        });
    }
    let profileId = operit1ActiveProfileId(parsed)?;
    Ok(Operit1CharacterMemoryBinding {
        memoryBindingMode: CharacterCardMemoryBindingMode::SHARED.to_string(),
        sharedMemoryId: Some(operit1SharedMemoryStoreId(&profileId)),
    })
}

#[allow(non_snake_case)]
fn collectOperit1CharacterMemoryProfileBindings(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeMap<String, String>, String> {
    let Some(preferences) = parsed.datastorePreferences.get(ENTRY_CHARACTER_CARDS) else {
        return Ok(BTreeMap::new());
    };
    let ids = optionalPreferenceStringSet(preferences, "character_card_list")?;
    let mut bindings = BTreeMap::new();
    for id in ids {
        let bindingMode = optionalPreferenceString(
            preferences,
            &format!("character_card_{id}_memory_profile_binding_mode"),
        )?;
        let profileId = if bindingMode.as_deref() == Some("FIXED_PROFILE") {
            requiredPreferenceString(
                preferences,
                &format!("character_card_{id}_memory_profile_id"),
                &format!("Operit1 角色卡固定用户偏好缺少配置 ID：{id}"),
            )?
            .to_string()
        } else {
            operit1ActiveProfileId(parsed)?
        };
        bindings.insert(id, profileId);
    }
    Ok(bindings)
}

#[allow(non_snake_case)]
fn collectOperit1MemoryProfileIds(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    ids.insert(OPERIT1_DEFAULT_PROFILE_ID.to_string());
    ids.extend(collectOperit1ObjectBoxProfileIds(parsed)?);
    for profileId in collectOperit1CharacterMemoryProfileBindings(parsed)?.values() {
        ids.insert(profileId.clone());
    }
    Ok(ids)
}

#[allow(non_snake_case)]
fn collectOperit1ObjectBoxProfileIds(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for entry in parsed.entries.keys() {
        if entry == ENTRY_OBJECTBOX_DEFAULT_DATA {
            ids.insert(OPERIT1_DEFAULT_PROFILE_ID.to_string());
            continue;
        }
        let Some(rest) = entry.strip_prefix("payload/files/objectbox_") else {
            continue;
        };
        let Some(profileId) = rest.strip_suffix("/data.mdb") else {
            continue;
        };
        validateOperit1ProfileId(profileId)?;
        ids.insert(profileId.to_string());
    }
    Ok(ids)
}

#[allow(non_snake_case)]
fn operit1ObjectBoxDataForProfile<'a>(
    parsed: &'a ParsedOperit1Snapshot,
    profileId: &str,
) -> Result<&'a [u8], String> {
    let entry = operit1ObjectBoxEntryForProfile(profileId);
    parsed
        .entries
        .get(&entry)
        .map(|bytes| bytes.as_slice())
        .ok_or_else(|| format!("Operit1 快照缺少记忆库文件：{entry}"))
}

#[allow(non_snake_case)]
fn operit1ObjectBoxEntryForProfile(profileId: &str) -> String {
    if profileId == OPERIT1_DEFAULT_PROFILE_ID {
        ENTRY_OBJECTBOX_DEFAULT_DATA.to_string()
    } else {
        format!("payload/files/objectbox_{profileId}/data.mdb")
    }
}

#[allow(non_snake_case)]
fn validateOperit1ProfileId(profileId: &str) -> Result<(), String> {
    if profileId.trim().is_empty()
        || profileId.contains('/')
        || profileId.contains('\\')
        || profileId.contains(':')
    {
        Err(format!("Operit1 用户偏好 ID 无效：{profileId}"))
    } else {
        Ok(())
    }
}

#[allow(non_snake_case)]
fn operit1SharedMemoryStoreId(profileId: &str) -> String {
    format!(
        "{OPERIT1_SHARED_MEMORY_STORE_ID_PREFIX}{}",
        sanitizeMemoryOwnerId(profileId)
    )
}

#[allow(non_snake_case)]
fn operit1SharedMemoryStoreName(
    parsed: &ParsedOperit1Snapshot,
    profileId: &str,
) -> Result<String, String> {
    let profiles = buildOperit1UserPreferenceProfiles(parsed)?;
    let profile = profiles
        .get(profileId)
        .ok_or_else(|| format!("Operit1 用户偏好缺少记忆库名称来源：{profileId}"))?;
    let profileName = profile.name.trim();
    if profileName.is_empty() {
        return Err(format!("Operit1 用户偏好名称为空：{profileId}"));
    }
    Ok(format!("Operit1 记忆库 - {profileName}"))
}

#[allow(non_snake_case)]
fn operit1ActiveProfileId(parsed: &ParsedOperit1Snapshot) -> Result<String, String> {
    let preferences = parsed
        .datastorePreferences
        .get(ENTRY_USER_PREFERENCES)
        .ok_or_else(|| format!("快照里没有 Operit1 用户偏好文件：{ENTRY_USER_PREFERENCES}"))?;
    let value = requiredPreferenceString(
        preferences,
        "active_profile_id",
        "Operit1 用户偏好缺少当前配置 ID",
    )?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err("Operit1 当前用户偏好 ID 为空".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
fn buildOperit1UserPreferenceProfiles(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeMap<String, Operit1UserPreferenceProfile>, String> {
    let Some(preferences) = parsed.datastorePreferences.get(ENTRY_USER_PREFERENCES) else {
        return Ok(BTreeMap::new());
    };
    let mut profiles = BTreeMap::new();
    for (key, value) in preferences {
        if key == "profile_list" {
            continue;
        }
        let Some(profileId) = key.strip_prefix("profile_") else {
            continue;
        };
        validateOperit1ProfileId(profileId)?;
        let raw = value
            .asString()
            .ok_or_else(|| format!("Operit1 用户偏好配置不是字符串：{profileId}"))?;
        let profile: Operit1UserPreferenceProfile = serde_json::from_str(raw)
            .map_err(|error| format!("Operit1 用户偏好配置格式不正确：{profileId}: {error}"))?;
        profiles.insert(profile.id.clone(), profile);
    }
    if let Some(profileIds) = optionalPreferenceStringList(preferences, "profile_list")? {
        for profileId in profileIds {
            validateOperit1ProfileId(&profileId)?;
            if !profiles.contains_key(&profileId) {
                return Err(format!("Operit1 用户偏好列表引用不存在的配置：{profileId}"));
            }
        }
    }
    Ok(profiles)
}

#[allow(non_snake_case)]
fn buildOperit2UserMarkdown(profile: &Operit1UserPreferenceProfile) -> Result<String, String> {
    let mut lines = Vec::new();
    lines.push(format!("## Operit1 用户偏好 - {}", profile.name.trim()));
    lines.push(String::new());
    pushMarkdownField(&mut lines, "配置 ID", &profile.id);
    if profile.birthDate > 0 {
        lines.push(format!(
            "- 出生日期：{}",
            epochMillisToLocalDateString(profile.birthDate)?
        ));
    }
    pushMarkdownField(&mut lines, "性别", &profile.gender);
    pushMarkdownField(&mut lines, "性格特点", &profile.personality);
    pushMarkdownField(&mut lines, "身份认同", &profile.identity);
    pushMarkdownField(&mut lines, "职业", &profile.occupation);
    pushMarkdownField(&mut lines, "期待的 AI 风格", &profile.aiStyle);
    Ok(lines.join("\n"))
}

#[allow(non_snake_case)]
fn pushMarkdownField(lines: &mut Vec<String>, label: &str, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        lines.push(format!("- {label}：{trimmed}"));
    }
}

#[allow(non_snake_case)]
fn appendSharedUserMarkdown(
    rootDir: &Path,
    profileId: &str,
    importedMarkdown: &str,
) -> Result<(), String> {
    let storeId = operit1SharedMemoryStoreId(profileId);
    let path = rootDir
        .join(DATA_MEMORY_SHARED_DIR_PATH)
        .join(sanitizeMemoryOwnerId(&storeId))
        .join("USER.md");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let current = if path.exists() {
        fs::read_to_string(&path).map_err(|error| error.to_string())?
    } else {
        "# USER\n\n".to_string()
    };
    let mut next = current.trim().to_string();
    let imported = importedMarkdown.trim();
    if !next.is_empty() {
        next.push_str("\n\n");
    }
    next.push_str(imported);
    next.push('\n');
    fs::write(path, next).map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn buildOperit2CharacterGroups(
    parsed: &ParsedOperit1Snapshot,
) -> Result<Vec<CharacterGroupCard>, String> {
    let Some(preferences) = parsed.datastorePreferences.get(ENTRY_CHARACTER_GROUPS) else {
        return Ok(Vec::new());
    };
    let ids = optionalPreferenceStringSet(preferences, "character_group_list")?;
    let mut groups = Vec::new();
    for id in ids {
        let raw = requiredPreferenceString(
            preferences,
            &format!("character_group_{id}_data"),
            &format!("Operit1 角色组缺少数据：{id}"),
        )?;
        let group: CharacterGroupCard = serde_json::from_str(raw)
            .map_err(|error| format!("Operit1 角色组格式不正确：{id}: {error}"))?;
        groups.push(CharacterGroupCard {
            id: group.id,
            name: group.name,
            description: group.description,
            members: group
                .members
                .into_iter()
                .map(|member| GroupMemberConfig {
                    characterCardId: member.characterCardId,
                    orderIndex: member.orderIndex,
                })
                .collect(),
            createdAt: group.createdAt,
            updatedAt: group.updatedAt,
        });
    }
    Ok(groups)
}

#[allow(non_snake_case)]
fn buildOperit2TtsConfig(parsed: &ParsedOperit1Snapshot) -> Result<Option<TtsConfig>, String> {
    let Some(preferences) = parsed.datastorePreferences.get(ENTRY_SPEECH_SERVICES) else {
        return Ok(None);
    };
    let serviceType = optionalPreferenceString(preferences, "tts_service_type")?;
    let Some(serviceType) = serviceType else {
        return Ok(None);
    };
    let serviceType = serviceType.trim();
    let now = currentTimeMillis();
    if serviceType == "SIMPLE_TTS" {
        return Ok(Some(TtsConfig {
            id: String::new(),
            name: "Operit1 系统 TTS".to_string(),
            providerType: TtsProviderType::SYSTEM_TTS.to_string(),
            endpoint: String::new(),
            apiKey: String::new(),
            model: String::new(),
            voice: String::new(),
            responseFormat: "wav".to_string(),
            speed: optionalPreferenceF64(preferences, "tts_speech_rate")?.unwrap_or(1.0),
            httpMethod: "POST".to_string(),
            requestBody: String::new(),
            contentType: "application/json".to_string(),
            headers: Vec::new(),
            responsePipeline: Vec::new(),
            createdAt: now,
            updatedAt: now,
        }));
    }
    let httpConfigRaw = requiredPreferenceString(
        preferences,
        "tts_http_config",
        &format!("Operit1 TTS 服务缺少 HTTP 配置：{serviceType}"),
    )?;
    let httpConfig: Operit1TtsHttpConfig = serde_json::from_str(httpConfigRaw)
        .map_err(|error| format!("Operit1 TTS HTTP 配置格式不正确：{error}"))?;
    Ok(Some(TtsConfig {
        id: String::new(),
        name: format!("Operit1 {serviceType}"),
        providerType: TtsProviderType::HTTP_TTS.to_string(),
        endpoint: httpConfig.urlTemplate,
        apiKey: httpConfig.apiKey,
        model: httpConfig.modelName,
        voice: httpConfig.voiceId,
        responseFormat: "wav".to_string(),
        speed: optionalPreferenceF64(preferences, "tts_speech_rate")?.unwrap_or(1.0),
        httpMethod: httpConfig.httpMethod,
        requestBody: httpConfig.requestBody,
        contentType: httpConfig.contentType,
        headers: httpConfig
            .headers
            .into_iter()
            .map(|(name, value)| TtsHttpHeader { name, value })
            .collect(),
        responsePipeline: httpConfig
            .responsePipeline
            .into_iter()
            .map(|step| TtsHttpResponsePipelineStep {
                stepType: step.stepType,
                path: step.path,
                headers: step
                    .headers
                    .into_iter()
                    .map(|(name, value)| TtsHttpHeader { name, value })
                    .collect(),
            })
            .collect(),
        createdAt: now,
        updatedAt: now,
    }))
}

#[allow(non_snake_case)]
fn importOperit1TtsConfig(manager: &TtsConfigManager, config: TtsConfig) -> Result<(), String> {
    let existing = manager
        .getAllTtsConfigs()
        .map_err(|error| format!("读取 Operit2 TTS 配置失败：{error}"))?
        .into_iter()
        .find(|existing| {
            existing.providerType == config.providerType
                && existing.endpoint == config.endpoint
                && existing.model == config.model
                && existing.voice == config.voice
        });
    let imported = match existing {
        Some(existing) => manager.updateTtsConfig(TtsConfig {
            id: existing.id,
            ..config
        }),
        None => manager.createTtsConfig(config),
    }
    .map_err(|error| format!("导入 Operit1 TTS 配置失败：{error}"))?;
    manager
        .setCurrentTtsConfigId(&imported.id)
        .map_err(|error| format!("设置 Operit1 TTS 配置失败：{error}"))?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1TtsHttpConfig {
    urlTemplate: String,
    apiKey: String,
    headers: HashMap<String, String>,
    #[serde(default = "defaultOperit1TtsHttpMethod")]
    httpMethod: String,
    #[serde(default)]
    requestBody: String,
    #[serde(default = "defaultOperit1TtsContentType")]
    contentType: String,
    #[serde(default)]
    voiceId: String,
    #[serde(default)]
    modelName: String,
    #[serde(default)]
    responsePipeline: Vec<Operit1TtsHttpResponsePipelineStep>,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1TtsHttpResponsePipelineStep {
    #[serde(alias = "type")]
    stepType: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[allow(non_snake_case)]
fn resolveOperit1CharacterChatModelId(
    parsed: &ParsedOperit1Snapshot,
    preferences: &HashMap<String, Operit1PreferenceValue>,
    cardId: &str,
) -> Result<Option<String>, String> {
    let bindingMode = optionalPreferenceString(
        preferences,
        &format!("character_card_{cardId}_chat_model_binding_mode"),
    )?;
    if bindingMode.as_deref() != Some("FIXED_CONFIG") {
        return Ok(None);
    }
    let configId = requiredPreferenceString(
        preferences,
        &format!("character_card_{cardId}_chat_model_config_id"),
        &format!("Operit1 角色卡固定模型缺少配置 ID：{cardId}"),
    )?;
    let modelIndex = optionalPreferenceI32(
        preferences,
        &format!("character_card_{cardId}_chat_model_index"),
    )?
    .ok_or_else(|| format!("Operit1 角色卡固定模型缺少模型索引：{cardId}"))?;
    let config = parsed.configById(configId)?;
    let modelIds = splitModelIds(&config.modelName);
    modelIds
        .get(modelIndex as usize)
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("Operit1 角色卡模型索引越界：{cardId}/{modelIndex}"))
}

#[allow(non_snake_case)]
fn optionalPreferenceString(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<String>, String> {
    preferences
        .get(key)
        .map(|value| {
            value
                .asString()
                .map(ToString::to_string)
                .ok_or_else(|| format!("Operit1 DataStore 键不是字符串：{key}"))
        })
        .transpose()
}

#[allow(non_snake_case)]
fn optionalPreferenceStringSet(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Vec<String>, String> {
    match preferences.get(key) {
        Some(value) => value
            .asStringSet()
            .map(|values| values.to_vec())
            .ok_or_else(|| format!("Operit1 DataStore 键不是字符串集合：{key}")),
        None => Ok(Vec::new()),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceStringList(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<Vec<String>>, String> {
    let Some(raw) = optionalPreferenceString(preferences, key)? else {
        return Ok(None);
    };
    serde_json::from_str::<Vec<String>>(&raw)
        .map(Some)
        .map_err(|error| format!("Operit1 DataStore 键不是字符串列表 JSON：{key}: {error}"))
}

#[allow(non_snake_case)]
fn optionalPreferenceBool(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<bool>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Boolean(value)) => Ok(Some(*value)),
        Some(value) => Err(format!("Operit1 DataStore 键不是布尔值：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceI32(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<i32>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Int(value)) => Ok(Some(*value)),
        Some(value) => Err(format!("Operit1 DataStore 键不是 i32：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceI64(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<i64>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Long(value)) => Ok(Some(*value)),
        Some(Operit1PreferenceValue::Int(value)) => Ok(Some(i64::from(*value))),
        Some(value) => Err(format!("Operit1 DataStore 键不是整数：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceF64(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<f64>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Float(value)) => Ok(Some(f64::from(*value))),
        Some(Operit1PreferenceValue::Double(value)) => Ok(Some(*value)),
        Some(value) => Err(format!("Operit1 DataStore 键不是浮点数：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn collectOperit1LegacyPromptTagIds(
    preferences: &HashMap<String, Operit1PreferenceValue>,
) -> Result<HashSet<String>, String> {
    let ids = optionalPreferenceStringSet(preferences, "prompt_tag_list")?;
    let mut legacyTagIds = HashSet::new();
    for id in ids {
        let isSystemTag =
            optionalPreferenceBool(preferences, &format!("prompt_tag_{id}_is_system_tag"))?
                .unwrap_or(false);
        let tagType = optionalPreferenceString(preferences, &format!("prompt_tag_{id}_tag_type"))?;
        if isSystemTag
            || tagType
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| value.starts_with("SYSTEM_"))
        {
            legacyTagIds.insert(id);
        }
    }
    Ok(legacyTagIds)
}

#[allow(non_snake_case)]
fn parseOperit1PromptTagType(value: Option<&str>) -> Result<TagType, String> {
    match value {
        Some("TONE") => Ok(TagType::TONE),
        Some("CHARACTER") => Ok(TagType::CHARACTER),
        Some("FUNCTION") => Ok(TagType::FUNCTION),
        Some("CUSTOM") | None => Ok(TagType::CUSTOM),
        Some(other) => Err(format!("Operit1 提示标签类型未知：{other}")),
    }
}

#[allow(non_snake_case)]
/// Builds a chat archive from the Operit1 SQLite database.
fn buildChatArchiveFromOperit1Database(
    path: &Path,
    fileImportPlan: &SnapshotFileImportPlan,
) -> Result<OperitChatArchive, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    if !sqliteTableExists(&connection, "chats")? || !sqliteTableExists(&connection, "messages")? {
        return Ok(OperitChatArchive {
            archiveType: ARCHIVE_TYPE.to_string(),
            formatVersion: CURRENT_FORMAT_VERSION,
            exportedAt: currentTimeMillis(),
            chats: Vec::new(),
        });
    }
    let mut chatStatement = connection
        .prepare(
            r#"
            SELECT id, title, createdAt, updatedAt, inputTokens, outputTokens,
                currentWindowSize, "group", displayOrder, workspace, parentChatId,
                characterCardName, locked
            FROM chats
            ORDER BY displayOrder ASC, updatedAt DESC
            "#,
        )
        .map_err(|error| error.to_string())?;
    let chatRows = chatStatement
        .query_map([], |row| {
            Ok(Operit1ChatRow {
                id: row.get(0)?,
                title: row.get(1)?,
                createdAt: row.get(2)?,
                updatedAt: row.get(3)?,
                inputTokens: row.get(4)?,
                outputTokens: row.get(5)?,
                currentWindowSize: row.get(6)?,
                group: row.get(7)?,
                displayOrder: row.get(8)?,
                workspace: row.get(9)?,
                parentChatId: row.get(10)?,
                characterCardName: row.get(11)?,
                locked: row.get::<_, i32>(12)? != 0,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut chats = Vec::new();
    for chat in chatRows {
        let messages = readOperit1Messages(&connection, &chat.id)?;
        if messages.is_empty() {
            continue;
        }
        chats.push(OperitArchivedChat {
            id: chat.id,
            title: chat.title,
            messages,
            createdAt: epochMillisToLocalDateTimeString(chat.createdAt)?,
            updatedAt: epochMillisToLocalDateTimeString(chat.updatedAt)?,
            inputTokens: chat.inputTokens,
            outputTokens: chat.outputTokens,
            currentWindowSize: chat.currentWindowSize,
            group: chat.group,
            displayOrder: chat.displayOrder,
            workspace: fileImportPlan.rewriteChatWorkspace(chat.workspace)?,
            parentChatId: chat.parentChatId,
            characterCardName: chat.characterCardName,
            characterGroupId: None,
            locked: chat.locked,
            pinned: false,
        });
    }
    Ok(OperitChatArchive {
        archiveType: ARCHIVE_TYPE.to_string(),
        formatVersion: CURRENT_FORMAT_VERSION,
        exportedAt: currentTimeMillis(),
        chats,
    })
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1ChatRow {
    id: String,
    title: String,
    createdAt: i64,
    updatedAt: i64,
    inputTokens: i32,
    outputTokens: i32,
    currentWindowSize: i32,
    group: Option<String>,
    displayOrder: i64,
    workspace: Option<String>,
    parentChatId: Option<String>,
    characterCardName: Option<String>,
    locked: bool,
}

#[allow(non_snake_case)]
/// Reads archived messages for one Operit1 chat row.
fn readOperit1Messages(
    connection: &Connection,
    chatId: &str,
) -> Result<Vec<OperitArchivedMessage>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT sender, content, timestamp, orderIndex, roleName, provider, modelName
            FROM messages
            WHERE chatId = ?1
            ORDER BY orderIndex ASC, timestamp ASC
            "#,
        )
        .map_err(|error| error.to_string())?;
    let messages = statement
        .query_map([chatId], |row| {
            let timestamp = row.get::<_, i64>(2)?;
            Ok(OperitArchivedMessage {
                baseMessage: ChatMessage {
                    sender: row.get(0)?,
                    content: row.get(1)?,
                    timestamp,
                    roleName: row.get(4)?,
                    selectedVariantIndex: 0,
                    variantCount: 1,
                    provider: row.get(5)?,
                    modelName: row.get(6)?,
                    inputTokens: 0,
                    outputTokens: 0,
                    cachedInputTokens: 0,
                    sentAt: 0,
                    outputDurationMs: 0,
                    waitDurationMs: 0,
                    completedAt: 0,
                    displayMode: ChatMessageDisplayMode::NORMAL,
                    isFavorite: false,
                    isVariantPreview: false,
                    contentStream: None,
                },
                variants: Vec::new(),
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(messages)
}

#[allow(non_snake_case)]
/// Returns whether the opened SQLite database contains the named table.
fn sqliteTableExists(connection: &Connection, tableName: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [tableName],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
/// Counts rows with a scalar count query.
fn queryCount(connection: &Connection, sql: &str) -> Result<i32, String> {
    connection
        .query_row(sql, [], |row| row.get::<_, i32>(0))
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn copyEntriesWithPrefix(
    rootDir: &Path,
    entries: &BTreeMap<String, Vec<u8>>,
    sourcePrefix: &str,
    targetPrefix: &str,
) -> Result<i32, String> {
    let mut count = 0;
    for (entry, bytes) in entries {
        if !entry.starts_with(sourcePrefix) || isDataStoreEntry(entry) {
            continue;
        }
        if sourcePrefix == ENTRY_FILES_PREFIX && entry.starts_with(ENTRY_WORKSPACE_FILES_PREFIX) {
            continue;
        }
        let relative = entry
            .strip_prefix(sourcePrefix)
            .ok_or_else(|| format!("快照资源路径前缀不匹配：{entry}"))?;
        validateRelativePath(relative)?;
        let target = rootDir.join(targetPrefix).join(relative);
        writeFile(&target, bytes)?;
        count += 1;
    }
    Ok(count)
}

#[allow(non_snake_case)]
fn copyWorkspaceEntries(
    entries: &BTreeMap<String, Vec<u8>>,
    fileImportPlan: &SnapshotFileImportPlan,
) -> Result<(i32, i32), String> {
    let mut workspaceIds = BTreeSet::new();
    let mut fileCount = 0;
    for (entry, bytes) in entries {
        let Some(relative) = entry.strip_prefix(ENTRY_WORKSPACE_FILES_PREFIX) else {
            continue;
        };
        validateRelativePath(relative)?;
        let (workspaceId, rest) = splitWorkspaceRelativePath(relative)?;
        workspaceIds.insert(workspaceId.to_string());
        if rest.is_empty() {
            fs::create_dir_all(fileImportPlan.workspaceRoot.join(workspaceId))
                .map_err(|error| error.to_string())?;
        } else {
            let target = fileImportPlan.workspaceRoot.join(workspaceId).join(rest);
            writeFile(&target, bytes)?;
            fileCount += 1;
        }
    }
    Ok((workspaceIds.len() as i32, fileCount))
}

#[allow(non_snake_case)]
fn splitWorkspaceRelativePath(relative: &str) -> Result<(&str, &str), String> {
    if let Some((workspaceId, rest)) = relative.split_once('/') {
        validateWorkspaceIdSegment(workspaceId)?;
        validateRelativePath(rest)?;
        return Ok((workspaceId, rest));
    }
    validateWorkspaceIdSegment(relative)?;
    Ok((relative, ""))
}

#[allow(non_snake_case)]
fn validateWorkspaceIdSegment(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
    {
        return Err(format!("Operit1 工作区 ID 无效：{value}"));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn workspaceVfsPath(workspaceId: &str, rest: &str) -> String {
    if rest.trim().is_empty() {
        format!("/app/workspaces/{workspaceId}")
    } else {
        format!("/app/workspaces/{workspaceId}/{rest}")
    }
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1MemoryRecord {
    id: u32,
    uuid: String,
    title: String,
    content: String,
    contentType: String,
    source: String,
    credibility: f32,
    importance: f32,
    folderPath: Option<String>,
    createdAt: i64,
    updatedAt: i64,
    isDocumentNode: bool,
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1MemoryLinkRecord {
    id: u32,
    type_: String,
    weight: f32,
    description: String,
    sourceId: u32,
    targetId: u32,
}

#[derive(Clone, Debug)]
struct Operit1MemoryTagRecord {
    id: u32,
    name: String,
}

#[allow(non_snake_case)]
fn buildMemoryExportDataFromOperit1ObjectBox(path: &Path) -> Result<MemoryExportData, String> {
    let environment = Environment::new()
        .set_max_dbs(64)
        .set_flags(EnvironmentFlags::NO_LOCK)
        .open(path)
        .map_err(|error| format!("打开 Operit1 记忆库失败：{error}"))?;
    let transaction = environment
        .begin_ro_txn()
        .map_err(|error| format!("读取 Operit1 记忆库失败：{error}"))?;
    let database = environment
        .open_db(None)
        .map_err(|error| format!("读取 Operit1 记忆库默认库失败：{error}"))?;
    let mut cursor = transaction
        .open_ro_cursor(database)
        .map_err(|error| format!("读取 Operit1 记忆库游标失败：{error}"))?;
    let mut memories = BTreeMap::<u32, Operit1MemoryRecord>::new();
    let mut links = BTreeMap::<u32, Operit1MemoryLinkRecord>::new();
    let mut tags = BTreeMap::<u32, Operit1MemoryTagRecord>::new();
    let mut memoryTagPairs = BTreeSet::<(u32, u32)>::new();

    for item in cursor.iter() {
        let (key, value) = item.map_err(|error| format!("读取 Operit1 记忆库记录失败：{error}"))?;
        if key.len() == 8 && key[0..4] == OPERIT1_OBJECTBOX_KEY_MEMORY && !value.is_empty() {
            let memory = parseOperit1MemoryRecord(value)?;
            memories.insert(memory.id, memory);
        } else if key.len() == 8 && key[0..4] == OPERIT1_OBJECTBOX_KEY_LINK && !value.is_empty() {
            let link = parseOperit1MemoryLinkRecord(value)?;
            links.insert(link.id, link);
        } else if key.len() == 8 && key[0..4] == OPERIT1_OBJECTBOX_KEY_TAG && !value.is_empty() {
            let tag = parseOperit1MemoryTagRecord(value)?;
            tags.insert(tag.id, tag);
        } else if key.len() == 16
            && key[0..4] == OPERIT1_OBJECTBOX_KEY_MEMORY_TAG_RELATION
            && value.is_empty()
        {
            let memoryId = readBigEndianU32(&key[8..12])?;
            let tagId = readBigEndianU32(&key[12..16])?;
            memoryTagPairs.insert((memoryId, tagId));
        }
    }

    let mut tagNamesByMemoryId = HashMap::<u32, Vec<String>>::new();
    for (memoryId, tagId) in memoryTagPairs {
        let tag = tags
            .get(&tagId)
            .ok_or_else(|| format!("Operit1 记忆标签关系引用不存在的标签：{tagId}"))?;
        tagNamesByMemoryId
            .entry(memoryId)
            .or_default()
            .push(tag.name.clone());
    }
    for tagNames in tagNamesByMemoryId.values_mut() {
        tagNames.sort();
        tagNames.dedup();
    }

    let mut memoryUuidById = HashMap::<u32, String>::new();
    let mut serializableMemories = Vec::new();
    for memory in memories.values() {
        if memory.isDocumentNode {
            continue;
        }
        memoryUuidById.insert(memory.id, memory.uuid.clone());
        serializableMemories.push(SerializableMemory {
            uuid: memory.uuid.clone(),
            title: memory.title.clone(),
            content: memory.content.clone(),
            contentType: memory.contentType.clone(),
            source: memory.source.clone(),
            credibility: memory.credibility,
            importance: memory.importance,
            folderPath: memory.folderPath.clone(),
            createdAt: memory.createdAt,
            updatedAt: memory.updatedAt,
            tagNames: tagNamesByMemoryId.remove(&memory.id).unwrap_or_default(),
        });
    }

    let mut seenLinks = BTreeSet::new();
    let mut serializableLinks = Vec::new();
    for link in links.values() {
        let sourceUuid = memoryUuidById
            .get(&link.sourceId)
            .ok_or_else(|| format!("Operit1 记忆链接引用不存在的源记忆：{}", link.sourceId))?;
        let targetUuid = memoryUuidById
            .get(&link.targetId)
            .ok_or_else(|| format!("Operit1 记忆链接引用不存在的目标记忆：{}", link.targetId))?;
        let key = (
            sourceUuid.clone(),
            targetUuid.clone(),
            link.type_.clone(),
            link.weight.to_bits(),
            link.description.clone(),
        );
        if !seenLinks.insert(key) {
            continue;
        }
        serializableLinks.push(SerializableLink {
            sourceUuid: sourceUuid.clone(),
            targetUuid: targetUuid.clone(),
            type_: link.type_.clone(),
            weight: link.weight,
            description: link.description.clone(),
        });
    }

    Ok(MemoryExportData {
        memories: serializableMemories,
        links: serializableLinks,
        exportDate: currentTimeMillis(),
        version: "1.0".to_string(),
    })
}

#[allow(non_snake_case)]
fn parseOperit1MemoryRecord(bytes: &[u8]) -> Result<Operit1MemoryRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryRecord {
        id: table.requiredU32(0, "Memory.id")?,
        uuid: table.requiredString(1, "Memory.uuid")?,
        title: table.requiredString(2, "Memory.title")?,
        content: table.requiredString(3, "Memory.content")?,
        contentType: table.requiredString(4, "Memory.contentType")?,
        source: table.requiredString(5, "Memory.source")?,
        credibility: table.requiredF32(6, "Memory.credibility")?,
        importance: table.requiredF32(7, "Memory.importance")?,
        createdAt: table.requiredI64(8, "Memory.createdAt")?,
        updatedAt: table.requiredI64(9, "Memory.updatedAt")?,
        isDocumentNode: table.optionalBool(13)?.unwrap_or(false),
        folderPath: table.optionalString(17)?,
    })
}

#[allow(non_snake_case)]
fn parseOperit1MemoryLinkRecord(bytes: &[u8]) -> Result<Operit1MemoryLinkRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryLinkRecord {
        id: table.requiredU32(0, "MemoryLink.id")?,
        type_: table.requiredString(1, "MemoryLink.type")?,
        weight: table.requiredF32(2, "MemoryLink.weight")?,
        description: table.requiredString(3, "MemoryLink.description")?,
        sourceId: table.requiredU32(6, "MemoryLink.sourceId")?,
        targetId: table.requiredU32(7, "MemoryLink.targetId")?,
    })
}

#[allow(non_snake_case)]
fn parseOperit1MemoryTagRecord(bytes: &[u8]) -> Result<Operit1MemoryTagRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryTagRecord {
        id: table.requiredU32(0, "MemoryTag.id")?,
        name: table.requiredString(1, "MemoryTag.name")?,
    })
}

struct FlatObjectBoxTable<'a> {
    bytes: &'a [u8],
    tableStart: usize,
    offsets: Vec<usize>,
}

impl<'a> FlatObjectBoxTable<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err("Operit1 ObjectBox 表内容过短".to_string());
        }
        let tableStart = readLittleEndianU32(&bytes[0..4])? as usize;
        if tableStart + 4 > bytes.len() {
            return Err("Operit1 ObjectBox 表根指针越界".to_string());
        }
        let vtableOffset = readLittleEndianI32(&bytes[tableStart..tableStart + 4])?;
        let vtableStart = if vtableOffset >= 0 {
            tableStart.checked_sub(vtableOffset as usize)
        } else {
            tableStart.checked_add((-vtableOffset) as usize)
        }
        .ok_or_else(|| "Operit1 ObjectBox vtable 偏移无效".to_string())?;
        if vtableStart + 4 > bytes.len() {
            return Err("Operit1 ObjectBox vtable 越界".to_string());
        }
        let vtableLength = readLittleEndianU16(&bytes[vtableStart..vtableStart + 2])? as usize;
        if vtableLength < 4 || vtableLength % 2 != 0 || vtableStart + vtableLength > bytes.len() {
            return Err("Operit1 ObjectBox vtable 长度无效".to_string());
        }
        let fieldCount = (vtableLength - 4) / 2;
        let mut offsets = Vec::new();
        for fieldIndex in 0..fieldCount {
            let start = vtableStart + 4 + fieldIndex * 2;
            offsets.push(readLittleEndianU16(&bytes[start..start + 2])? as usize);
        }
        Ok(Self {
            bytes,
            tableStart,
            offsets,
        })
    }

    #[allow(non_snake_case)]
    fn fieldAbs(&self, index: usize) -> Option<usize> {
        let relative = *self.offsets.get(index)?;
        if relative == 0 {
            return None;
        }
        self.tableStart
            .checked_add(relative)
            .filter(|position| *position < self.bytes.len())
    }

    #[allow(non_snake_case)]
    fn requiredU32(&self, index: usize, label: &str) -> Result<u32, String> {
        let abs = self
            .fieldAbs(index)
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))?;
        self.readU32Abs(abs, label)
    }

    #[allow(non_snake_case)]
    fn requiredI64(&self, index: usize, label: &str) -> Result<i64, String> {
        let abs = self
            .fieldAbs(index)
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))?;
        if abs + 8 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        readLittleEndianI64(&self.bytes[abs..abs + 8])
    }

    #[allow(non_snake_case)]
    fn requiredF32(&self, index: usize, label: &str) -> Result<f32, String> {
        let abs = self
            .fieldAbs(index)
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))?;
        if abs + 4 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        Ok(f32::from_le_bytes(
            self.bytes[abs..abs + 4]
                .try_into()
                .map_err(|_| format!("Operit1 ObjectBox 字段无效：{label}"))?,
        ))
    }

    #[allow(non_snake_case)]
    fn requiredString(&self, index: usize, label: &str) -> Result<String, String> {
        self.optionalString(index)?
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))
    }

    #[allow(non_snake_case)]
    fn optionalString(&self, index: usize) -> Result<Option<String>, String> {
        let Some(abs) = self.fieldAbs(index) else {
            return Ok(None);
        };
        if abs + 4 > self.bytes.len() {
            return Err("Operit1 ObjectBox 字符串指针越界".to_string());
        }
        let relative = readLittleEndianI32(&self.bytes[abs..abs + 4])?;
        if relative <= 0 {
            return Err("Operit1 ObjectBox 字符串偏移无效".to_string());
        }
        let vectorStart = abs
            .checked_add(relative as usize)
            .ok_or_else(|| "Operit1 ObjectBox 字符串偏移溢出".to_string())?;
        if vectorStart + 4 > self.bytes.len() {
            return Err("Operit1 ObjectBox 字符串长度越界".to_string());
        }
        let length = readLittleEndianU32(&self.bytes[vectorStart..vectorStart + 4])? as usize;
        let start = vectorStart + 4;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "Operit1 ObjectBox 字符串长度溢出".to_string())?;
        if end > self.bytes.len() {
            return Err("Operit1 ObjectBox 字符串内容越界".to_string());
        }
        String::from_utf8(self.bytes[start..end].to_vec())
            .map(Some)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn optionalBool(&self, index: usize) -> Result<Option<bool>, String> {
        let Some(abs) = self.fieldAbs(index) else {
            return Ok(None);
        };
        if abs >= self.bytes.len() {
            return Err("Operit1 ObjectBox 布尔字段越界".to_string());
        }
        Ok(Some(self.bytes[abs] != 0))
    }

    #[allow(non_snake_case)]
    fn readU32Abs(&self, abs: usize, label: &str) -> Result<u32, String> {
        if abs + 4 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        readLittleEndianU32(&self.bytes[abs..abs + 4])
    }
}

#[allow(non_snake_case)]
fn readBigEndianU32(bytes: &[u8]) -> Result<u32, String> {
    Ok(u32::from_be_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u32 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn readLittleEndianU16(bytes: &[u8]) -> Result<u16, String> {
    Ok(u16::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u16 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn readLittleEndianU32(bytes: &[u8]) -> Result<u32, String> {
    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u32 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn readLittleEndianI32(bytes: &[u8]) -> Result<i32, String> {
    Ok(i32::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox i32 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn readLittleEndianI64(bytes: &[u8]) -> Result<i64, String> {
    Ok(i64::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox i64 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn writeTempFile(path: &Path, bytes: &[u8]) -> Result<(), String> {
    writeFile(path, bytes)
}

#[allow(non_snake_case)]
fn writeFile(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut file = fs::File::create(path).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn validateSnapshotEntryPath(path: &str) -> Result<(), String> {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') || path.contains('\\') {
        return Err(format!("Operit1 快照包含非法路径：{path}"));
    }
    validateRelativePath(path)
}

#[allow(non_snake_case)]
fn validateRelativePath(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.split('/').any(|segment| {
            segment.is_empty() || segment == "." || segment == ".." || segment.contains(':')
        })
    {
        return Err(format!("Operit1 快照包含非法相对路径：{path}"));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn epochMillisToLocalDateTimeString(value: i64) -> Result<String, String> {
    let datetime = chrono::Local
        .timestamp_millis_opt(value)
        .single()
        .ok_or_else(|| format!("Operit1 聊天时间戳无效：{value}"))?;
    Ok(datetime
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S%.3f")
        .to_string())
}

#[allow(non_snake_case)]
fn epochMillisToLocalDateString(value: i64) -> Result<String, String> {
    let datetime = chrono::Local
        .timestamp_millis_opt(value)
        .single()
        .ok_or_else(|| format!("Operit1 用户偏好日期无效：{value}"))?;
    Ok(datetime.naive_local().format("%Y-%m-%d").to_string())
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}

#[allow(non_snake_case)]
fn requiredPreferenceString<'a>(
    preferences: &'a HashMap<String, Operit1PreferenceValue>,
    key: &str,
    missingMessage: &str,
) -> Result<&'a str, String> {
    let value = preferences
        .get(key)
        .ok_or_else(|| missingMessage.to_string())?;
    value
        .asString()
        .ok_or_else(|| format!("Operit1 DataStore 键不是字符串：{key}"))
}

#[allow(non_snake_case)]
fn decodeDataStorePreferences(
    bytes: &[u8],
) -> Result<HashMap<String, Operit1PreferenceValue>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut preferences = HashMap::new();
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        if fieldNumber != 1 || wireType != 2 {
            return Err(format!(
                "DataStore Preferences 出现未知字段：{fieldNumber}/{wireType}"
            ));
        }
        let entryBytes = decoder.readLengthDelimited()?;
        if let Some((key, value)) = decodePreferenceEntry(entryBytes)? {
            preferences.insert(key, value);
        }
    }
    Ok(preferences)
}

#[allow(non_snake_case)]
fn decodePreferenceEntry(bytes: &[u8]) -> Result<Option<(String, Operit1PreferenceValue)>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut key = None;
    let mut value = None;
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        match (fieldNumber, wireType) {
            (1, 2) => key = Some(decoder.readString()?),
            (2, 2) => value = decodePreferenceValue(decoder.readLengthDelimited()?)?,
            _ => decoder.skipField(wireType)?,
        }
    }
    let key = key.ok_or_else(|| "DataStore PreferenceEntry 缺少 key".to_string())?;
    Ok(value.map(|value| (key, value)))
}

#[allow(non_snake_case)]
fn decodePreferenceValue(bytes: &[u8]) -> Result<Option<Operit1PreferenceValue>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut value = None;
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        match (fieldNumber, wireType) {
            (1, 0) => value = Some(Operit1PreferenceValue::Boolean(decoder.readVarint()? != 0)),
            (2, 5) => {
                value = Some(Operit1PreferenceValue::Float(f32::from_le_bytes(
                    decoder.readFixed32()?.to_le_bytes(),
                )))
            }
            (3, 1) => {
                value = Some(Operit1PreferenceValue::Double(f64::from_le_bytes(
                    decoder.readFixed64()?.to_le_bytes(),
                )))
            }
            (4, 0) => {
                value = Some(Operit1PreferenceValue::Int(decodeInt32Varint(
                    decoder.readVarint()?,
                )))
            }
            (5, 2) => value = Some(Operit1PreferenceValue::String(decoder.readString()?)),
            (6, 2) => {
                value = Some(Operit1PreferenceValue::StringSet(
                    decodePreferenceStringSet(decoder.readLengthDelimited()?)?,
                ));
            }
            (7, 0) => {
                value = Some(Operit1PreferenceValue::Long(decodeInt64Varint(
                    decoder.readVarint()?,
                )))
            }
            _ => decoder.skipField(wireType)?,
        }
    }
    Ok(value)
}

#[allow(non_snake_case)]
fn decodePreferenceStringSet(bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut values = Vec::new();
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        match (fieldNumber, wireType) {
            (1, 2) => values.push(decoder.readString()?),
            _ => decoder.skipField(wireType)?,
        }
    }
    Ok(values)
}

#[allow(non_snake_case)]
fn decodeInt32Varint(raw: u64) -> i32 {
    raw as u32 as i32
}

#[allow(non_snake_case)]
fn decodeInt64Varint(raw: u64) -> i64 {
    raw as i64
}

struct ProtoDecoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ProtoDecoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    #[allow(non_snake_case)]
    fn isComplete(&self) -> bool {
        self.position == self.bytes.len()
    }

    #[allow(non_snake_case)]
    fn readTag(&mut self) -> Result<(u64, u64), String> {
        let tag = self.readVarint()?;
        Ok((tag >> 3, tag & 0x07))
    }

    #[allow(non_snake_case)]
    fn readLengthDelimited(&mut self) -> Result<&'a [u8], String> {
        let length = self.readVarint()? as usize;
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| "DataStore protobuf 长度溢出".to_string())?;
        if end > self.bytes.len() {
            return Err("DataStore protobuf 内容不完整".to_string());
        }
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    #[allow(non_snake_case)]
    fn readString(&mut self) -> Result<String, String> {
        String::from_utf8(self.readLengthDelimited()?.to_vec()).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn readFixed32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.readFixedBytes::<4>()?))
    }

    #[allow(non_snake_case)]
    fn readFixed64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.readFixedBytes::<8>()?))
    }

    #[allow(non_snake_case)]
    fn skipField(&mut self, wireType: u64) -> Result<(), String> {
        match wireType {
            0 => {
                self.readVarint()?;
                Ok(())
            }
            1 => self.skipBytes(8),
            2 => {
                self.readLengthDelimited()?;
                Ok(())
            }
            5 => self.skipBytes(4),
            _ => Err(format!("DataStore protobuf 出现未知 wire type：{wireType}")),
        }
    }

    #[allow(non_snake_case)]
    fn skipBytes(&mut self, count: usize) -> Result<(), String> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| "DataStore protobuf 长度溢出".to_string())?;
        if end > self.bytes.len() {
            return Err("DataStore protobuf 内容不完整".to_string());
        }
        self.position = end;
        Ok(())
    }

    #[allow(non_snake_case)]
    fn readFixedBytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self
            .position
            .checked_add(N)
            .ok_or_else(|| "DataStore protobuf 长度溢出".to_string())?;
        if end > self.bytes.len() {
            return Err("DataStore protobuf 内容不完整".to_string());
        }
        let bytes = self.bytes[self.position..end]
            .try_into()
            .map_err(|_| "DataStore protobuf 固定长度字段不完整".to_string())?;
        self.position = end;
        Ok(bytes)
    }

    #[allow(non_snake_case)]
    fn readVarint(&mut self) -> Result<u64, String> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            if self.position >= self.bytes.len() {
                return Err("DataStore protobuf varint 不完整".to_string());
            }
            let byte = self.bytes[self.position];
            self.position += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("DataStore protobuf varint 无效".to_string())
    }
}

#[allow(non_snake_case)]
fn parseCustomParameterValue(value: &str) -> Result<Value, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("Operit1 自定义参数值不是合法 JSON：{error}"))
}

#[allow(non_snake_case)]
fn parseParameterValueType(
    value: &str,
) -> Result<operit_model::ModelParameter::ParameterValueType, String> {
    match value {
        "INT" => Ok(operit_model::ModelParameter::ParameterValueType::INT),
        "FLOAT" => Ok(operit_model::ModelParameter::ParameterValueType::FLOAT),
        "STRING" => Ok(operit_model::ModelParameter::ParameterValueType::STRING),
        "BOOLEAN" => Ok(operit_model::ModelParameter::ParameterValueType::BOOLEAN),
        "OBJECT" => Ok(operit_model::ModelParameter::ParameterValueType::OBJECT),
        other => Err(format!("未知模型参数值类型：{other}")),
    }
}

#[allow(non_snake_case)]
fn parseParameterCategory(value: &str) -> Result<ParameterCategory, String> {
    match value {
        "GENERATION" => Ok(ParameterCategory::GENERATION),
        "CREATIVITY" => Ok(ParameterCategory::CREATIVITY),
        "REPETITION" => Ok(ParameterCategory::REPETITION),
        "OTHER" => Ok(ParameterCategory::OTHER),
        other => Err(format!("未知模型参数分类：{other}")),
    }
}

#[allow(non_snake_case)]
fn defaultKeyRotationMode() -> String {
    "ROUND_ROBIN".to_string()
}

#[allow(non_snake_case)]
fn defaultOperit1ConfigId() -> String {
    "default".to_string()
}

#[allow(non_snake_case)]
fn defaultOperit1TtsHttpMethod() -> String {
    "GET".to_string()
}

#[allow(non_snake_case)]
fn defaultOperit1TtsContentType() -> String {
    "application/json".to_string()
}

#[allow(non_snake_case)]
fn defaultTrue() -> bool {
    true
}

#[allow(non_snake_case)]
fn defaultApiKeyAvailabilityStatus() -> ApiKeyAvailabilityStatus {
    ApiKeyAvailabilityStatus::UNTESTED
}

#[allow(non_snake_case)]
fn defaultCustomParameters() -> String {
    "[]".to_string()
}

#[allow(non_snake_case)]
fn defaultCustomHeaders() -> String {
    "{}".to_string()
}

#[allow(non_snake_case)]
fn defaultMaxTokens() -> i32 {
    StandardModelParameters::DEFAULT_MAX_TOKENS
}

#[allow(non_snake_case)]
fn defaultTemperature() -> f32 {
    StandardModelParameters::DEFAULT_TEMPERATURE
}

#[allow(non_snake_case)]
fn defaultTopP() -> f32 {
    StandardModelParameters::DEFAULT_TOP_P
}

#[allow(non_snake_case)]
fn defaultRepetitionPenalty() -> f32 {
    StandardModelParameters::DEFAULT_REPETITION_PENALTY
}

#[allow(non_snake_case)]
fn defaultMaxContextLength() -> f32 {
    ModelConfigDefaults::DEFAULT_MAX_CONTEXT_LENGTH
}

#[allow(non_snake_case)]
fn defaultSummaryTokenThreshold() -> f32 {
    ModelConfigDefaults::DEFAULT_SUMMARY_TOKEN_THRESHOLD
}

#[allow(non_snake_case)]
fn defaultEnableSummary() -> bool {
    ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY
}

#[allow(non_snake_case)]
fn defaultEnableSummaryByMessageCount() -> bool {
    ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY_BY_MESSAGE_COUNT
}

#[allow(non_snake_case)]
fn defaultSummaryMessageCountThreshold() -> i32 {
    ModelConfigDefaults::DEFAULT_SUMMARY_MESSAGE_COUNT_THRESHOLD
}

#[allow(non_snake_case)]
fn defaultThreadCount() -> i32 {
    4
}

#[allow(non_snake_case)]
fn defaultLlamaContextSize() -> i32 {
    2048
}

#[allow(non_snake_case)]
fn defaultLlamaBatchSize() -> i32 {
    512
}

#[allow(non_snake_case)]
fn defaultLlamaKvUnified() -> bool {
    true
}
