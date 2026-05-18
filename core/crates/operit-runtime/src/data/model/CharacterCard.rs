use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct CharacterCardToolAccessConfig {
    pub enabled: bool,
    pub allowedBuiltinTools: Vec<String>,
    pub allowedPackages: Vec<String>,
    pub allowedSkills: Vec<String>,
    pub allowedMcpServers: Vec<String>,
}

impl CharacterCardToolAccessConfig {
    pub fn normalized(&self) -> CharacterCardToolAccessConfig {
        CharacterCardToolAccessConfig {
            enabled: self.enabled,
            allowedBuiltinTools: Self::normalizeEntries(&self.allowedBuiltinTools),
            allowedPackages: Self::normalizeEntries(&self.allowedPackages),
            allowedSkills: Self::normalizeEntries(&self.allowedSkills),
            allowedMcpServers: Self::normalizeEntries(&self.allowedMcpServers),
        }
    }

    pub fn hasExternalSelections(&self) -> bool {
        !self.allowedPackages.is_empty()
            || !self.allowedSkills.is_empty()
            || !self.allowedMcpServers.is_empty()
    }

    fn normalizeEntries(values: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        for value in values {
            let trimmed = value.trim();
            if !trimmed.is_empty() && !result.iter().any(|entry| entry == trimmed) {
                result.push(trimmed.to_string());
            }
        }
        result
    }
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
    pub attachedTagIds: Vec<String>,
    pub advancedCustomPrompt: String,
    pub marks: String,
    pub chatModelBindingMode: String,
    pub chatModelConfigId: Option<String>,
    pub chatModelIndex: i32,
    pub memoryProfileBindingMode: String,
    pub memoryProfileId: Option<String>,
    pub toolAccessConfig: CharacterCardToolAccessConfig,
    pub isDefault: bool,
    pub createdAt: i64,
    pub updatedAt: i64,
}

pub struct CharacterCardChatModelBindingMode;

impl CharacterCardChatModelBindingMode {
    pub const FOLLOW_GLOBAL: &'static str = "FOLLOW_GLOBAL";
    pub const FIXED_CONFIG: &'static str = "FIXED_CONFIG";

    pub fn normalize(mode: Option<&str>) -> String {
        if mode == Some(Self::FIXED_CONFIG) {
            Self::FIXED_CONFIG.to_string()
        } else {
            Self::FOLLOW_GLOBAL.to_string()
        }
    }
}

pub struct CharacterCardMemoryProfileBindingMode;

impl CharacterCardMemoryProfileBindingMode {
    pub const FOLLOW_GLOBAL: &'static str = "FOLLOW_GLOBAL";
    pub const FIXED_PROFILE: &'static str = "FIXED_PROFILE";

    pub fn normalize(mode: Option<&str>) -> String {
        if mode == Some(Self::FIXED_PROFILE) {
            Self::FIXED_PROFILE.to_string()
        } else {
            Self::FOLLOW_GLOBAL.to_string()
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TavernCharacterCard {
    #[serde(default)]
    pub spec: String,
    #[serde(default)]
    pub spec_version: String,
    pub data: TavernCharacterData,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TavernCharacterData {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub first_mes: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub mes_example: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub post_history_instructions: String,
    #[serde(default)]
    pub alternate_greetings: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub character_version: String,
    #[serde(default)]
    pub extensions: Option<TavernExtensions>,
    #[serde(default)]
    pub character_book: Option<TavernCharacterBook>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TavernExtensions {
    #[serde(default)]
    pub chub: Option<TavernChubExtension>,
    #[serde(default)]
    pub depth_prompt: Option<TavernDepthPrompt>,
    #[serde(default)]
    pub operit: Option<OperitTavernExtension>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitTavernExtension {
    #[serde(default = "default_operit_character_card_schema")]
    pub schema: String,
    pub character_card: OperitCharacterCardPayload,
}

fn default_operit_character_card_schema() -> String {
    "operit_character_card_v1".to_string()
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct OperitCharacterCardPayload {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub characterSetting: String,
    #[serde(default)]
    pub openingStatement: String,
    #[serde(default)]
    pub otherContent: String,
    #[serde(default)]
    pub otherContentChat: String,
    #[serde(default)]
    pub otherContentVoice: String,
    #[serde(default)]
    pub attachedTagIds: Vec<String>,
    #[serde(default)]
    pub attachedTags: Vec<OperitAttachedTagPayload>,
    #[serde(default)]
    pub advancedCustomPrompt: String,
    #[serde(default)]
    pub marks: String,
    #[serde(default = "default_character_chat_model_binding_mode")]
    pub chatModelBindingMode: String,
    #[serde(default)]
    pub chatModelConfigId: Option<String>,
    #[serde(default)]
    pub chatModelIndex: i32,
    #[serde(default = "default_character_memory_profile_binding_mode")]
    pub memoryProfileBindingMode: String,
    #[serde(default)]
    pub memoryProfileId: Option<String>,
    #[serde(default)]
    pub toolAccessConfig: Option<CharacterCardToolAccessConfig>,
}

fn default_character_chat_model_binding_mode() -> String {
    CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string()
}

fn default_character_memory_profile_binding_mode() -> String {
    CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL.to_string()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitAttachedTagPayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub promptContent: String,
    #[serde(default = "default_attached_tag_type")]
    pub tagType: String,
}

impl Default for OperitAttachedTagPayload {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            promptContent: String::new(),
            tagType: default_attached_tag_type(),
        }
    }
}

fn default_attached_tag_type() -> String {
    "CUSTOM".to_string()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TavernChubExtension {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub full_path: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub expressions: Option<String>,
    #[serde(default)]
    pub alt_expressions: HashMap<String, String>,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub related_lorebooks: Vec<String>,
}

impl Default for TavernChubExtension {
    fn default() -> Self {
        Self {
            id: 0,
            preset: None,
            full_path: String::new(),
            extensions: Vec::new(),
            expressions: None,
            alt_expressions: HashMap::new(),
            background_image: None,
            related_lorebooks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TavernDepthPrompt {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub depth: i32,
    #[serde(default)]
    pub prompt: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TavernCharacterBook {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scan_depth: i32,
    #[serde(default)]
    pub token_budget: i32,
    #[serde(default)]
    pub recursive_scanning: bool,
    #[serde(default)]
    pub extensions: HashMap<String, Value>,
    #[serde(default)]
    pub entries: Vec<TavernBookEntry>,
}

impl Default for TavernCharacterBook {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            scan_depth: 0,
            token_budget: 0,
            recursive_scanning: false,
            extensions: HashMap::new(),
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TavernBookEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub secondary_keys: Vec<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub insertion_order: i32,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub selective: bool,
    #[serde(default)]
    pub constant: bool,
    #[serde(default)]
    pub position: String,
    #[serde(default)]
    pub extensions: HashMap<String, Value>,
    #[serde(default = "default_probability")]
    pub probability: i32,
    #[serde(default)]
    pub selectiveLogic: i32,
}

impl Default for TavernBookEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            keys: Vec::new(),
            secondary_keys: Vec::new(),
            content: String::new(),
            enabled: true,
            insertion_order: 0,
            case_sensitive: false,
            priority: 0,
            id: 0,
            comment: String::new(),
            selective: false,
            constant: false,
            position: String::new(),
            extensions: HashMap::new(),
            probability: 100,
            selectiveLogic: 0,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_probability() -> i32 {
    100
}
