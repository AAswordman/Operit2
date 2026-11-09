//! Source character and memory formats projected by the Operit1 snapshot importer.
//! This module contains source records and readers, with no business repositories or writers.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct CharacterCardToolAccessConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allowedBuiltinTools: Vec<String>,
    #[serde(default)]
    pub allowedPackages: Vec<String>,
    #[serde(default)]
    pub allowedSkills: Vec<String>,
    #[serde(default)]
    pub allowedMcpServers: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CharacterSharedMemoryMount {
    pub sharedMemoryId: String,
    pub readable: bool,
    pub writable: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CharacterCard {
    pub id: String,
    pub name: String,
    pub description: String,
    pub characterSetting: String,
    pub openingStatement: String,
    pub otherContentChat: String,
    pub otherContentVoice: String,
    pub avatarUri: Option<String>,
    pub attachedTagIds: Vec<String>,
    pub advancedCustomPrompt: String,
    pub marks: String,
    pub chatModelBindingMode: String,
    pub chatModelId: Option<String>,
    #[serde(default)]
    pub chatProviderId: Option<String>,
    pub ttsConfigId: Option<String>,
    #[serde(default = "default_character_memory_binding_mode")]
    pub memoryBindingMode: String,
    #[serde(default)]
    pub sharedMemoryId: Option<String>,
    pub sharedMemoryMounts: Vec<CharacterSharedMemoryMount>,
    pub toolAccessConfig: CharacterCardToolAccessConfig,
    pub isDefault: bool,
    pub createdAt: i64,
    pub updatedAt: i64,
}

pub struct CharacterCardChatModelBindingMode;
impl CharacterCardChatModelBindingMode {
    pub const FOLLOW_GLOBAL: &'static str = "FOLLOW_GLOBAL";
    pub const FIXED_MODEL: &'static str = "FIXED_MODEL";
}

pub struct CharacterCardMemoryBindingMode;
impl CharacterCardMemoryBindingMode {
    pub const CHARACTER: &'static str = "CHARACTER";
    pub const SHARED: &'static str = "SHARED";
}

/// Reproduces the memory binding default declared by the released source card format.
fn default_character_memory_binding_mode() -> String {
    CharacterCardMemoryBindingMode::CHARACTER.to_string()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GroupMemberConfig {
    pub characterCardId: String,
    pub orderIndex: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CharacterGroupCard {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: Vec<GroupMemberConfig>,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromptTag {
    pub id: String,
    pub name: String,
    pub description: String,
    pub promptContent: String,
    pub tagType: TagType,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TagType {
    TONE,
    CHARACTER,
    FUNCTION,
    CUSTOM,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SharedMemoryStore {
    pub id: String,
    pub name: String,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Memory {
    pub id: i64,
    pub uuid: String,
    pub title: String,
    pub content: String,
    pub contentType: String,
    pub source: String,
    pub credibility: f32,
    pub importance: f32,
    pub documentPath: Option<String>,
    pub isDocumentNode: bool,
    pub chunkIndexFilePath: Option<String>,
    pub folderPath: Option<String>,
    pub createdAt: i64,
    pub updatedAt: i64,
    pub lastAccessedAt: i64,
    pub tags: Vec<MemoryTag>,
    pub properties: Vec<MemoryProperty>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MemoryTag {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemoryLink {
    pub id: i64,
    pub sourceMemoryId: i64,
    pub targetMemoryId: i64,
    pub type_: String,
    pub weight: f32,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MemoryProperty {
    pub id: i64,
    pub key: String,
    pub value: String,
}

/// Persisted document paragraph. UUID binding survives import/sync ID remapping.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DocumentChunk {
    pub id: i64,
    pub memoryUuid: String,
    pub chunkIndex: i32,
    pub content: String,
}
/// Stores one owner-scoped request for deferred memory extraction.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemoryAutoSaveCandidate {
    pub id: i64,
    pub chatId: String,
    pub triggerMessageTimestamp: i64,
    pub createdAt: i64,
    pub updatedAt: i64,
    pub status: String,
    pub attemptCount: i32,
    pub lastError: String,
    pub sourceType: String,
}

impl MemoryAutoSaveCandidate {
    pub const SOURCE_TYPE_REPLY_FINALIZED_AUTO: &'static str = "reply_finalized_auto";
    pub const SOURCE_TYPE_SELECTED_USER_MESSAGE: &'static str = "selected_user_message";
}

/// Kotlin memory-space settings, addressed by Operit2's character/shared owner key.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct MemorySettings {
    pub autoSaveIntervalMinutes: i32,
    pub nextAutoSaveRunAtMs: i64,
    pub memoryExtractionCustomRules: String,
    pub profileAutoUpdateEnabled: bool,
    pub profileAutoUpdateLocked: bool,
    pub cloudEmbeddingEnabled: bool,
    pub cloudEmbeddingEndpoint: String,
    pub cloudEmbeddingApiKey: String,
    pub cloudEmbeddingModel: String,
}
impl Default for MemorySettings {
    /// Reproduces the settings defaults of the released source format.
    fn default() -> Self {
        Self {
            autoSaveIntervalMinutes: 5,
            nextAutoSaveRunAtMs: 0,
            memoryExtractionCustomRules: String::new(),
            profileAutoUpdateEnabled: true,
            profileAutoUpdateLocked: false,
            cloudEmbeddingEnabled: false,
            cloudEmbeddingEndpoint: String::new(),
            cloudEmbeddingApiKey: String::new(),
            cloudEmbeddingModel: String::new(),
        }
    }
}
impl MemorySettings {
    /// Applies the released source format's persisted settings normalization.
    pub fn normalized(mut self) -> Self {
        self.autoSaveIntervalMinutes = self.autoSaveIntervalMinutes.clamp(1, 30);
        self.nextAutoSaveRunAtMs = self.nextAutoSaveRunAtMs.max(0);
        self.cloudEmbeddingEndpoint = self.cloudEmbeddingEndpoint.trim().to_string();
        self.cloudEmbeddingModel = self.cloudEmbeddingModel.trim().to_string();
        self.cloudEmbeddingApiKey = self.cloudEmbeddingApiKey.trim().to_string();
        self
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MemoryScoreMode {
    BALANCED,
    KEYWORD_FIRST,
    SEMANTIC_FIRST,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemorySearchConfig {
    pub scoreMode: MemoryScoreMode,
    pub keywordWeight: f32,
    pub tagWeight: f32,
    pub vectorWeight: f32,
    pub edgeWeight: f32,
}

impl Default for MemorySearchConfig {
    /// Reproduces the search defaults of the released source format.
    fn default() -> Self {
        Self {
            scoreMode: MemoryScoreMode::BALANCED,
            keywordWeight: 10.0,
            tagWeight: 0.0,
            vectorWeight: 0.0,
            edgeWeight: 0.4,
        }
    }
}

impl MemorySearchConfig {
    /// Applies the released source format's persisted search-weight normalization.
    pub fn normalized(mut self) -> Self {
        for weight in [
            &mut self.keywordWeight,
            &mut self.tagWeight,
            &mut self.vectorWeight,
            &mut self.edgeWeight,
        ] {
            *weight = if weight.is_finite() {
                weight.max(0.0)
            } else {
                0.0
            };
        }
        self
    }
}

/// Encodes a source owner identifier with the released directory naming scheme.
pub fn sanitizeMemoryOwnerId(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
