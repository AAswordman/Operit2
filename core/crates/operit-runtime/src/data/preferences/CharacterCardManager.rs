use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::data::model::CharacterCard::{
    CharacterCard, CharacterCardChatModelBindingMode, CharacterCardMemoryProfileBindingMode,
    CharacterCardToolAccessConfig, OperitAttachedTagPayload, OperitCharacterCardPayload,
    OperitTavernExtension, TavernCharacterCard, TavernCharacterData, TavernExtensions,
};
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::model::PromptTag::{PromptTag, TagType};
use crate::data::preferences::CharacterCardBilingualData::CharacterCardBilingualData;
use crate::data::preferences::PromptTagManager::PromptTagManager;

#[derive(Clone)]
pub struct CharacterCardManager {
    dataStore: PreferencesDataStore,
    tagManager: PromptTagManager,
}

struct ImportedTagResult {
    idMap: HashMap<String, String>,
    importedIds: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CharacterCardsBackupFile {
    #[serde(default, rename = "characterCards")]
    characterCards: Vec<CharacterCard>,
    #[serde(default, rename = "promptTags")]
    promptTags: Vec<PromptTag>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum CharacterCardsBackupInput {
    BackupFile(CharacterCardsBackupFile),
    CharacterCards(Vec<CharacterCard>),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CharacterCardImportResult {
    pub new: i32,
    pub updated: i32,
    pub skipped: i32,
    pub total: i32,
}

impl CharacterCardManager {
    #[allow(non_snake_case)]
    pub const DEFAULT_CHARACTER_CARD_ID: &str = "default_character";
    #[allow(non_snake_case)]
    pub const DEFAULT_CHARACTER_NAME: &str = "Operit";

    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(
                paths.root_dir().join("character_cards.preferences.json"),
            ),
            tagManager: PromptTagManager::new(paths),
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    #[allow(non_snake_case)]
    fn CHARACTER_CARD_LIST() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("character_card_list")
    }

    #[allow(non_snake_case)]
    fn ACTIVE_CHARACTER_CARD_ID() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("active_character_card_id")
    }

    #[allow(non_snake_case)]
    pub fn characterCardListFlow(&self) -> Flow<Vec<String>> {
        self.dataStore
            .dataFlow()
            .map(|preferences| Self::readCardList(&preferences))
    }

    #[allow(non_snake_case)]
    pub fn observeActiveCharacterCardId(&self) -> Flow<Option<String>> {
        self.dataStore
            .dataFlow()
            .map(|preferences| preferences.get(&Self::ACTIVE_CHARACTER_CARD_ID()).cloned())
    }

    #[allow(non_snake_case)]
    pub fn getCharacterCardFlow(&self, id: &str) -> Flow<CharacterCard> {
        let manager = self.clone();
        let id = id.to_string();
        self.dataStore
            .dataFlow()
            .map(move |preferences| manager.getCharacterCardFromPreferences(&preferences, &id))
    }

    #[allow(non_snake_case)]
    pub fn getCharacterCard(&self, id: &str) -> Result<CharacterCard, PreferencesDataStoreError> {
        self.getCharacterCardFlow(id).first()
    }

    #[allow(non_snake_case)]
    fn getCharacterCardFromPreferences(
        &self,
        preferences: &Preferences,
        id: &str,
    ) -> CharacterCard {
        CharacterCard {
            id: id.to_string(),
            name: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_name")))
                .cloned()
                .unwrap_or_else(|| Self::DEFAULT_CHARACTER_NAME.to_string()),
            description: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_description"
                )))
                .cloned()
                .unwrap_or_default(),
            characterSetting: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_character_setting"
                )))
                .cloned()
                .unwrap_or_default(),
            openingStatement: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_opening_statement"
                )))
                .cloned()
                .unwrap_or_default(),
            otherContentChat: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_other_content_chat"
                )))
                .cloned()
                .unwrap_or_default(),
            otherContentVoice: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_other_content_voice"
                )))
                .cloned()
                .unwrap_or_default(),
            attachedTagIds: readJsonVec(
                preferences,
                &format!("character_card_{id}_attached_tag_ids"),
            ),
            advancedCustomPrompt: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_advanced_custom_prompt"
                )))
                .cloned()
                .unwrap_or_default(),
            marks: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_marks")))
                .cloned()
                .unwrap_or_default(),
            chatModelBindingMode: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_chat_model_binding_mode"
                )))
                .map(|value| CharacterCardChatModelBindingMode::normalize(Some(value)))
                .unwrap_or_else(|| CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string()),
            chatModelConfigId: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_chat_model_config_id"
                )))
                .cloned(),
            chatModelIndex: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_chat_model_index"
                )))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(0),
            memoryProfileBindingMode: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_memory_profile_binding_mode"
                )))
                .map(|value| CharacterCardMemoryProfileBindingMode::normalize(Some(value)))
                .unwrap_or_else(|| {
                    CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL.to_string()
                }),
            memoryProfileId: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_memory_profile_id"
                )))
                .cloned(),
            toolAccessConfig: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_tool_access_config_json"
                )))
                .and_then(|raw| serde_json::from_str::<CharacterCardToolAccessConfig>(raw).ok())
                .unwrap_or_default()
                .normalized(),
            isDefault: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_is_default"
                )))
                .map(|value| value == "true")
                .unwrap_or(id == Self::DEFAULT_CHARACTER_CARD_ID),
            createdAt: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_created_at"
                )))
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_else(currentTimeMillis),
            updatedAt: preferences
                .get(&stringPreferencesKey(&format!(
                    "character_card_{id}_updated_at"
                )))
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_else(currentTimeMillis),
        }
    }

    #[allow(non_snake_case)]
    pub fn createCharacterCard(
        &self,
        card: CharacterCard,
    ) -> Result<String, PreferencesDataStoreError> {
        let id = if card.isDefault {
            Self::DEFAULT_CHARACTER_CARD_ID.to_string()
        } else if card.id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            card.id.clone()
        };
        let now = currentTimeMillis();
        self.dataStore.edit(|preferences| {
            let mut currentList = Self::readCardList(preferences);
            if !currentList.contains(&id) {
                currentList.push(id.clone());
            }
            currentList.sort();
            currentList.dedup();
            Self::writeCardList(preferences, currentList);
            self.writeCard(preferences, &card, &id, now);
            if card.isDefault || preferences.get(&Self::ACTIVE_CHARACTER_CARD_ID()).is_none() {
                preferences.set(&Self::ACTIVE_CHARACTER_CARD_ID(), id.clone());
            }
        })?;
        Ok(id)
    }

    #[allow(non_snake_case)]
    pub fn updateCharacterCard(
        &self,
        card: CharacterCard,
    ) -> Result<(), PreferencesDataStoreError> {
        let now = currentTimeMillis();
        self.dataStore.edit(|preferences| {
            self.writeCard(preferences, &card, &card.id, now);
        })
    }

    #[allow(non_snake_case)]
    pub fn deleteCharacterCard(&self, id: &str) -> Result<(), PreferencesDataStoreError> {
        if id == Self::DEFAULT_CHARACTER_CARD_ID {
            return Ok(());
        }
        self.dataStore.edit(|preferences| {
            let mut currentList = Self::readCardList(preferences);
            currentList.retain(|item| item != id);
            Self::writeCardList(preferences, currentList);
            self.removeCardKeys(preferences, id);
            if preferences.get(&Self::ACTIVE_CHARACTER_CARD_ID()) == Some(&id.to_string()) {
                preferences.remove(&Self::ACTIVE_CHARACTER_CARD_ID());
            }
        })
    }

    #[allow(non_snake_case)]
    pub fn setActiveCharacterCard(&self, id: &str) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.set(&Self::ACTIVE_CHARACTER_CARD_ID(), id.to_string());
        })
    }

    #[allow(non_snake_case)]
    pub fn clearActiveCharacterCard(&self) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.remove(&Self::ACTIVE_CHARACTER_CARD_ID());
        })
    }

    #[allow(non_snake_case)]
    pub fn getAllCharacterCards(&self) -> Result<Vec<CharacterCard>, PreferencesDataStoreError> {
        let ids = self.characterCardListFlow().first()?;
        Ok(ids
            .into_iter()
            .map(|id| self.getCharacterCard(&id))
            .filter_map(Result::ok)
            .collect())
    }

    #[allow(non_snake_case)]
    pub fn findCharacterCardByName(
        &self,
        name: &str,
    ) -> Result<Option<CharacterCard>, PreferencesDataStoreError> {
        let normalized = name.trim();
        Ok(self
            .getAllCharacterCards()?
            .into_iter()
            .find(|card| card.name.trim() == normalized))
    }

    #[allow(non_snake_case)]
    pub fn initializeIfNeeded(&self) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            let currentList = preferences
                .get(&Self::CHARACTER_CARD_LIST())
                .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok());
            if currentList
                .as_ref()
                .map(|list| list.is_empty())
                .unwrap_or(true)
            {
                preferences.set(
                    &Self::CHARACTER_CARD_LIST(),
                    serde_json::to_string(&vec![Self::DEFAULT_CHARACTER_CARD_ID.to_string()])
                        .expect("character card list must serialize"),
                );
                self.setupDefaultCharacterCard(preferences, Self::DEFAULT_CHARACTER_CARD_ID);
            }
        })?;
        self.tagManager.removeLegacyBuiltInTags()?;
        self.removeDeletedTagReferencesFromCharacterCards()?;
        self.migrateLegacyOtherContentToChat()?;
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn resetDefaultCharacterCard(&self) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            self.setupDefaultCharacterCard(preferences, Self::DEFAULT_CHARACTER_CARD_ID);
        })
    }

    #[allow(non_snake_case)]
    pub fn combinePrompts(
        &self,
        characterCardId: &str,
        additionalTagIds: Vec<String>,
        promptFunctionType: PromptFunctionType,
    ) -> Result<String, PreferencesDataStoreError> {
        let characterCard = self.getCharacterCard(characterCardId)?;
        let mut allTagIds = Vec::new();
        for tagId in characterCard
            .attachedTagIds
            .into_iter()
            .chain(additionalTagIds.into_iter())
        {
            if !allTagIds.contains(&tagId) {
                allTagIds.push(tagId);
            }
        }
        let mut parts = Vec::new();
        if !characterCard.characterSetting.trim().is_empty() {
            parts.push(characterCard.characterSetting.trim().to_string());
        }
        let otherContent = match promptFunctionType {
            PromptFunctionType::VOICE => characterCard.otherContentVoice.trim().to_string(),
            PromptFunctionType::CHAT => characterCard.otherContentChat.trim().to_string(),
        };
        if !otherContent.is_empty() {
            parts.push(otherContent);
        }
        for tagId in allTagIds {
            if let Ok(tag) = self.tagManager.getPromptTagFlow(&tagId).first() {
                if !tag.promptContent.trim().is_empty() {
                    parts.push(tag.promptContent.trim().to_string());
                }
            }
        }
        if !characterCard.advancedCustomPrompt.trim().is_empty() {
            parts.push(characterCard.advancedCustomPrompt.trim().to_string());
        }
        Ok(parts.join("\n\n").trim().to_string())
    }

    #[allow(non_snake_case)]
    pub fn exportAllCharacterCardsToBackupContent(&self) -> Result<String, String> {
        let cards = self
            .getAllCharacterCards()
            .map_err(|error| error.to_string())?;
        let mut referencedTagIds = Vec::new();
        for card in &cards {
            for tagId in &card.attachedTagIds {
                if !referencedTagIds.contains(tagId) {
                    referencedTagIds.push(tagId.clone());
                }
            }
        }
        let promptTags = referencedTagIds
            .iter()
            .filter_map(|tagId| self.tagManager.getPromptTagFlow(tagId).first().ok())
            .collect::<Vec<_>>();
        let backup = CharacterCardsBackupFile {
            characterCards: cards,
            promptTags,
        };
        serde_json::to_string_pretty(&backup)
            .map_err(|error| format!("导出角色卡备份失败：{error}"))
    }

    #[allow(non_snake_case)]
    pub fn importAllCharacterCardsFromBackupContent(
        &self,
        jsonContent: &str,
    ) -> Result<CharacterCardImportResult, String> {
        if jsonContent.trim().is_empty() {
            return Err("角色卡备份内容不能为空".to_string());
        }
        let backupInput = serde_json::from_str::<CharacterCardsBackupInput>(jsonContent)
            .map_err(|error| format!("角色卡备份 JSON 格式错误：{error}"))?;
        let backup = match backupInput {
            CharacterCardsBackupInput::BackupFile(backup) => backup,
            CharacterCardsBackupInput::CharacterCards(characterCards) => CharacterCardsBackupFile {
                characterCards,
                promptTags: Vec::new(),
            },
        };
        let existingIds = self
            .characterCardListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        let importedTagIdMap = self.importOrReuseBackupPromptTags(&backup.promptTags)?;
        let mut newCount = 0;
        let mut updatedCount = 0;
        let mut skippedCount = 0;

        for card in backup.characterCards {
            if card.id.trim().is_empty() || card.name.trim().is_empty() {
                skippedCount += 1;
                continue;
            }
            let finalCard = CharacterCard {
                isDefault: card.id == Self::DEFAULT_CHARACTER_CARD_ID,
                attachedTagIds: remapAttachedTagIds(&card.attachedTagIds, &importedTagIdMap),
                chatModelBindingMode: CharacterCardChatModelBindingMode::normalize(Some(
                    &card.chatModelBindingMode,
                )),
                chatModelConfigId: card
                    .chatModelConfigId
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                chatModelIndex: card.chatModelIndex.max(0),
                memoryProfileBindingMode: CharacterCardMemoryProfileBindingMode::normalize(Some(
                    &card.memoryProfileBindingMode,
                )),
                memoryProfileId: card
                    .memoryProfileId
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                toolAccessConfig: card.toolAccessConfig.normalized(),
                ..card
            };
            if existingIds.contains(&finalCard.id) {
                updatedCount += 1;
            } else {
                newCount += 1;
            }
            self.upsertCharacterCardWithId(finalCard)
                .map_err(|error| error.to_string())?;
        }

        Ok(CharacterCardImportResult {
            new: newCount,
            updated: updatedCount,
            skipped: skippedCount,
            total: newCount + updatedCount,
        })
    }

    #[allow(non_snake_case)]
    pub fn createCharacterCardFromTavernJson(&self, jsonString: &str) -> Result<String, String> {
        let tavernCard = serde_json::from_str::<TavernCharacterCard>(jsonString)
            .map_err(|error| format!("角色卡 JSON 格式错误：{error}"))?;
        if tavernCard.data.name.trim().is_empty() {
            return Err("角色卡名称不能为空".to_string());
        }

        let mut worldBookTagId = None;
        if let Some(book) = &tavernCard.data.character_book {
            if !book.entries.is_empty() {
                let worldBookContent = book
                    .entries
                    .iter()
                    .filter(|entry| !entry.content.trim().is_empty())
                    .map(|entry| format!("[{}]\n{}", entry.name, entry.content))
                    .collect::<Vec<_>>()
                    .join("\n\n")
                    .trim()
                    .to_string();
                if !worldBookContent.is_empty() {
                    worldBookTagId = Some(
                        self.tagManager
                            .createOrReusePromptTag(
                                CharacterCardBilingualData::getWorldBookTagName(
                                    false,
                                    &tavernCard.data.name,
                                ),
                                CharacterCardBilingualData::getWorldBookTagDescription(
                                    false,
                                    &tavernCard.data.name,
                                ),
                                worldBookContent,
                                TagType::FUNCTION,
                            )
                            .map_err(|error| error.to_string())?,
                    );
                }
            }
        }

        let operitPayload = tavernCard
            .data
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.operit.as_ref())
            .filter(|operit| operit.schema == "operit_character_card_v1")
            .map(|operit| &operit.character_card);

        let mut characterCard = if let Some(payload) = operitPayload {
            let importedOperitTags = self.importOrReuseOperitTags(&payload.attachedTags)?;
            let attachedTagIds = if !payload.attachedTagIds.is_empty() {
                remapAttachedTagIds(&payload.attachedTagIds, &importedOperitTags.idMap)
            } else if !importedOperitTags.importedIds.is_empty() {
                importedOperitTags.importedIds
            } else {
                importedOperitTags.idMap.values().cloned().fold(
                    Vec::<String>::new(),
                    |mut entries, id| {
                        if !entries.contains(&id) {
                            entries.push(id);
                        }
                        entries
                    },
                )
            };
            self.characterCardFromOperitPayload(payload, attachedTagIds)
        } else {
            self.convertTavernCardToCharacterCard(&tavernCard)
        };

        if let Some(tagId) = worldBookTagId {
            if !characterCard.attachedTagIds.contains(&tagId) {
                characterCard.attachedTagIds.push(tagId);
            }
        }
        self.createCharacterCard(characterCard)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    pub fn exportCharacterCardToTavernJson(&self, characterCardId: &str) -> Result<String, String> {
        let card = self
            .getCharacterCard(characterCardId)
            .map_err(|error| error.to_string())?;
        let attachedTags = card
            .attachedTagIds
            .iter()
            .filter_map(|tagId| self.tagManager.getPromptTagFlow(tagId).first().ok())
            .collect::<Vec<_>>();
        let tagNames = attachedTags
            .iter()
            .map(|tag| tag.name.clone())
            .collect::<Vec<_>>();
        let operitExt = OperitTavernExtension {
            schema: "operit_character_card_v1".to_string(),
            character_card: OperitCharacterCardPayload {
                name: card.name.clone(),
                description: card.description.clone(),
                characterSetting: card.characterSetting.clone(),
                openingStatement: card.openingStatement.clone(),
                otherContent: card.otherContentChat.clone(),
                otherContentChat: card.otherContentChat.clone(),
                otherContentVoice: card.otherContentVoice.clone(),
                attachedTagIds: card.attachedTagIds.clone(),
                attachedTags: attachedTags
                    .iter()
                    .map(|tag| OperitAttachedTagPayload {
                        id: tag.id.clone(),
                        name: tag.name.clone(),
                        description: tag.description.clone(),
                        promptContent: tag.promptContent.clone(),
                        tagType: tagTypeName(&tag.tagType).to_string(),
                    })
                    .collect(),
                advancedCustomPrompt: card.advancedCustomPrompt.clone(),
                marks: card.marks.clone(),
                chatModelBindingMode: CharacterCardChatModelBindingMode::normalize(Some(
                    &card.chatModelBindingMode,
                )),
                chatModelConfigId: card
                    .chatModelConfigId
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                chatModelIndex: card.chatModelIndex.max(0),
                memoryProfileBindingMode: CharacterCardMemoryProfileBindingMode::normalize(Some(
                    &card.memoryProfileBindingMode,
                )),
                memoryProfileId: card
                    .memoryProfileId
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                toolAccessConfig: Some(card.toolAccessConfig.normalized()),
            },
        };
        let tavernCard = TavernCharacterCard {
            spec: "chara_card_v2".to_string(),
            spec_version: "2.0".to_string(),
            data: TavernCharacterData {
                name: card.name,
                description: card.description,
                personality: String::new(),
                first_mes: card.openingStatement,
                avatar: String::new(),
                mes_example: card.otherContentChat,
                scenario: String::new(),
                creator_notes: card.marks,
                system_prompt: card.characterSetting,
                post_history_instructions: card.advancedCustomPrompt,
                alternate_greetings: Vec::new(),
                tags: tagNames,
                creator: String::new(),
                character_version: String::new(),
                extensions: Some(TavernExtensions {
                    operit: Some(operitExt),
                    ..Default::default()
                }),
                character_book: None,
            },
        };
        serde_json::to_string(&tavernCard).map_err(|error| format!("导出角色卡失败：{error}"))
    }

    #[allow(non_snake_case)]
    fn characterCardFromOperitPayload(
        &self,
        payload: &OperitCharacterCardPayload,
        attachedTagIds: Vec<String>,
    ) -> CharacterCard {
        let now = currentTimeMillis();
        CharacterCard {
            id: String::new(),
            name: payload.name.clone(),
            description: payload.description.clone(),
            characterSetting: payload.characterSetting.clone(),
            openingStatement: payload.openingStatement.clone(),
            otherContentChat: if payload.otherContentChat.trim().is_empty() {
                payload.otherContent.clone()
            } else {
                payload.otherContentChat.clone()
            },
            otherContentVoice: payload.otherContentVoice.clone(),
            attachedTagIds,
            advancedCustomPrompt: payload.advancedCustomPrompt.clone(),
            marks: payload.marks.clone(),
            chatModelBindingMode: CharacterCardChatModelBindingMode::normalize(Some(
                &payload.chatModelBindingMode,
            )),
            chatModelConfigId: payload
                .chatModelConfigId
                .clone()
                .filter(|value| !value.trim().is_empty()),
            chatModelIndex: payload.chatModelIndex.max(0),
            memoryProfileBindingMode: CharacterCardMemoryProfileBindingMode::normalize(Some(
                &payload.memoryProfileBindingMode,
            )),
            memoryProfileId: payload
                .memoryProfileId
                .clone()
                .filter(|value| !value.trim().is_empty()),
            toolAccessConfig: payload
                .toolAccessConfig
                .clone()
                .unwrap_or_default()
                .normalized(),
            isDefault: false,
            createdAt: now,
            updatedAt: now,
        }
    }

    #[allow(non_snake_case)]
    fn convertTavernCardToCharacterCard(&self, tavernCard: &TavernCharacterCard) -> CharacterCard {
        let data = &tavernCard.data;
        let characterSetting = joinNonEmpty(
            vec![
                labeledBlock(
                    CharacterCardBilingualData::getCharacterDescriptionLabel(false),
                    &data.description,
                ),
                labeledBlock(
                    CharacterCardBilingualData::getPersonalityLabel(false),
                    &data.personality,
                ),
                labeledBlock(
                    CharacterCardBilingualData::getScenarioLabel(false),
                    &data.scenario,
                ),
            ],
            "\n\n",
        );
        let alternateGreetings = if data.alternate_greetings.is_empty() {
            String::new()
        } else {
            let mut text = CharacterCardBilingualData::getAlternateGreetingsLabel(false);
            text.push('\n');
            for (index, greeting) in data.alternate_greetings.iter().enumerate() {
                text.push_str(&format!("{}. {}\n", index + 1, greeting));
            }
            text
        };
        let otherContentChat = joinNonEmpty(
            vec![
                labeledBlock(
                    CharacterCardBilingualData::getDialogueExampleLabel(false),
                    &data.mes_example,
                ),
                labeledBlock(
                    CharacterCardBilingualData::getSystemPromptLabel(false),
                    &data.system_prompt,
                ),
                labeledBlock(
                    CharacterCardBilingualData::getPostHistoryInstructionsLabel(false),
                    &data.post_history_instructions,
                ),
                alternateGreetings,
            ],
            "\n\n",
        );
        let advancedCustomPrompt = data
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.depth_prompt.as_ref())
            .map(|depthPrompt| {
                labeledBlock(
                    CharacterCardBilingualData::getDepthPromptLabel(false),
                    &depthPrompt.prompt,
                )
            })
            .unwrap_or_default();
        let description = if data.tags.is_empty() {
            String::new()
        } else {
            let mut text = CharacterCardBilingualData::getTagsLabel(false);
            text.push_str(
                &data
                    .tags
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            if data.tags.len() > 5 {
                text.push_str(&CharacterCardBilingualData::getEtAlLabel(false));
                text.push_str(&data.tags.len().to_string());
            }
            text
        };
        let mut marks = CharacterCardBilingualData::getSourceLabel(false);
        if !data.creator.trim().is_empty() {
            marks.push_str(&CharacterCardBilingualData::getAuthorLabel(false));
            marks.push_str(&data.creator);
            marks.push('\n');
        }
        if !data.creator_notes.trim().is_empty() {
            marks.push_str(&CharacterCardBilingualData::getAuthorNotesLabel(false));
            marks.push_str(&data.creator_notes);
            marks.push_str("\n\n");
        }
        if !data.character_version.trim().is_empty() {
            marks.push_str(&CharacterCardBilingualData::getVersionLabel(false));
            marks.push_str(&data.character_version);
            marks.push('\n');
        }
        if !data.tags.is_empty() {
            marks.push_str(&CharacterCardBilingualData::getOriginalTagsLabel(false));
            marks.push_str(&data.tags.join(", "));
            marks.push('\n');
        }
        if !tavernCard.spec.trim().is_empty() {
            marks.push_str(&CharacterCardBilingualData::getFormatLabel(false));
            marks.push_str(&tavernCard.spec);
            if !tavernCard.spec_version.trim().is_empty() {
                marks.push_str(&format!(" v{}", tavernCard.spec_version));
            }
            marks.push('\n');
        }
        let now = currentTimeMillis();
        CharacterCard {
            id: String::new(),
            name: data.name.clone(),
            description,
            characterSetting,
            openingStatement: data.first_mes.clone(),
            otherContentChat,
            otherContentVoice: String::new(),
            attachedTagIds: Vec::new(),
            advancedCustomPrompt,
            marks: marks.trim().to_string(),
            chatModelBindingMode: CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string(),
            chatModelConfigId: None,
            chatModelIndex: 0,
            memoryProfileBindingMode: CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL
                .to_string(),
            memoryProfileId: None,
            toolAccessConfig: CharacterCardToolAccessConfig::default(),
            isDefault: false,
            createdAt: now,
            updatedAt: now,
        }
    }

    #[allow(non_snake_case)]
    fn importOrReuseOperitTags(
        &self,
        exportedTags: &[OperitAttachedTagPayload],
    ) -> Result<ImportedTagResult, String> {
        let mut idMap = HashMap::new();
        let mut importedIds = Vec::new();
        for exportedTag in exportedTags {
            if let Some(localTagId) = self.importOrReuseTag(
                &exportedTag.id,
                &exportedTag.name,
                &exportedTag.description,
                &exportedTag.promptContent,
                &exportedTag.tagType,
            )? {
                if !importedIds.contains(&localTagId) {
                    importedIds.push(localTagId.clone());
                }
                if !exportedTag.id.trim().is_empty() {
                    idMap.insert(exportedTag.id.clone(), localTagId);
                }
            }
        }
        Ok(ImportedTagResult { idMap, importedIds })
    }

    #[allow(non_snake_case)]
    fn importOrReuseBackupPromptTags(
        &self,
        exportedTags: &[PromptTag],
    ) -> Result<HashMap<String, String>, String> {
        let mut idMap = HashMap::new();
        for tag in exportedTags {
            if tag.id.trim().is_empty()
                || tag.name.trim().is_empty()
                || tag.promptContent.trim().is_empty()
            {
                continue;
            }
            let importedId = self
                .tagManager
                .createOrReusePromptTag(
                    tag.name.clone(),
                    tag.description.clone(),
                    tag.promptContent.clone(),
                    tag.tagType.clone(),
                )
                .map_err(|error| error.to_string())?;
            idMap.insert(tag.id.clone(), importedId);
        }
        Ok(idMap)
    }

    #[allow(non_snake_case)]
    fn importOrReuseTag(
        &self,
        exportedTagId: &str,
        name: &str,
        description: &str,
        promptContent: &str,
        tagTypeNameValue: &str,
    ) -> Result<Option<String>, String> {
        if name.trim().is_empty()
            && description.trim().is_empty()
            && promptContent.trim().is_empty()
        {
            return Ok(None);
        }
        if let Some(existingTag) = self
            .tagManager
            .findTagWithSameContent(promptContent)
            .map_err(|error| error.to_string())?
        {
            return Ok(Some(existingTag.id));
        }
        let tagName = if name.trim().is_empty() {
            if exportedTagId.trim().is_empty() {
                "Imported Tag".to_string()
            } else {
                exportedTagId.to_string()
            }
        } else {
            name.to_string()
        };
        let tagType = parseTagTypeName(tagTypeNameValue);
        self.tagManager
            .createPromptTag(
                tagName,
                description.to_string(),
                promptContent.to_string(),
                tagType,
            )
            .map(Some)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn writeCard(&self, preferences: &mut Preferences, card: &CharacterCard, id: &str, now: i64) {
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_name")),
            card.name.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_description")),
            card.description.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_character_setting")),
            card.characterSetting.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_opening_statement")),
            card.openingStatement.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_other_content_chat")),
            card.otherContentChat.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_other_content_voice")),
            card.otherContentVoice.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_attached_tag_ids")),
            serde_json::to_string(&card.attachedTagIds).expect("attached tag ids must serialize"),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_advanced_custom_prompt")),
            card.advancedCustomPrompt.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_marks")),
            card.marks.clone(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_chat_model_binding_mode")),
            card.chatModelBindingMode.clone(),
        );
        if let Some(value) = &card.chatModelConfigId {
            preferences.set(
                &stringPreferencesKey(&format!("character_card_{id}_chat_model_config_id")),
                value.clone(),
            );
        } else {
            preferences.remove(&stringPreferencesKey(&format!(
                "character_card_{id}_chat_model_config_id"
            )));
        }
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_chat_model_index")),
            card.chatModelIndex.max(0).to_string(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_memory_profile_binding_mode")),
            card.memoryProfileBindingMode.clone(),
        );
        if let Some(value) = &card.memoryProfileId {
            preferences.set(
                &stringPreferencesKey(&format!("character_card_{id}_memory_profile_id")),
                value.clone(),
            );
        } else {
            preferences.remove(&stringPreferencesKey(&format!(
                "character_card_{id}_memory_profile_id"
            )));
        }
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_tool_access_config_json")),
            serde_json::to_string(&card.toolAccessConfig.normalized())
                .expect("tool access config must serialize"),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_is_default")),
            card.isDefault.to_string(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_created_at")),
            card.createdAt.to_string(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_updated_at")),
            now.to_string(),
        );
    }

    #[allow(non_snake_case)]
    fn setupDefaultCharacterCard(&self, preferences: &mut Preferences, id: &str) {
        let now = currentTimeMillis();
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_name")),
            Self::DEFAULT_CHARACTER_NAME.to_string(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_description")),
            CharacterCardBilingualData::getDefaultDescription(false),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_character_setting")),
            CharacterCardBilingualData::getDefaultCharacterSetting(false),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_opening_statement")),
            String::new(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_other_content_chat")),
            CharacterCardBilingualData::getDefaultOtherContentChat(false),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_other_content_voice")),
            CharacterCardBilingualData::getDefaultOtherContentVoice(false),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_attached_tag_ids")),
            serde_json::to_string(&Vec::<String>::new()).expect("attached tag ids must serialize"),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_advanced_custom_prompt")),
            String::new(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_marks")),
            String::new(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_chat_model_binding_mode")),
            CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string(),
        );
        preferences.remove(&stringPreferencesKey(&format!(
            "character_card_{id}_chat_model_config_id"
        )));
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_chat_model_index")),
            "0".to_string(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_memory_profile_binding_mode")),
            CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL.to_string(),
        );
        preferences.remove(&stringPreferencesKey(&format!(
            "character_card_{id}_memory_profile_id"
        )));
        preferences.remove(&stringPreferencesKey(&format!(
            "character_card_{id}_tool_access_config_json"
        )));
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_is_default")),
            true.to_string(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_created_at")),
            now.to_string(),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_updated_at")),
            now.to_string(),
        );
    }

    #[allow(non_snake_case)]
    fn removeCardKeys(&self, preferences: &mut Preferences, id: &str) {
        for suffix in [
            "name",
            "description",
            "character_setting",
            "opening_statement",
            "other_content_chat",
            "other_content_voice",
            "attached_tag_ids",
            "advanced_custom_prompt",
            "marks",
            "chat_model_binding_mode",
            "chat_model_config_id",
            "chat_model_index",
            "memory_profile_binding_mode",
            "memory_profile_id",
            "tool_access_config_json",
            "is_default",
            "created_at",
            "updated_at",
        ] {
            preferences.remove(&stringPreferencesKey(&format!(
                "character_card_{id}_{suffix}"
            )));
        }
    }

    #[allow(non_snake_case)]
    fn upsertCharacterCardWithId(
        &self,
        card: CharacterCard,
    ) -> Result<(), PreferencesDataStoreError> {
        let id = card.id.clone();
        if id.trim().is_empty() {
            return Ok(());
        }
        self.dataStore.edit(|preferences| {
            let mut currentList = Self::readCardList(preferences);
            if !currentList.contains(&id) {
                currentList.push(id.clone());
            }
            currentList.sort();
            currentList.dedup();
            Self::writeCardList(preferences, currentList);
            self.writeCard(preferences, &card, &id, card.updatedAt);
            if preferences.get(&Self::ACTIVE_CHARACTER_CARD_ID()).is_none() {
                preferences.set(
                    &Self::ACTIVE_CHARACTER_CARD_ID(),
                    Self::DEFAULT_CHARACTER_CARD_ID.to_string(),
                );
            }
        })
    }

    #[allow(non_snake_case)]
    #[allow(non_snake_case)]
    fn readCardList(preferences: &Preferences) -> Vec<String> {
        preferences
            .get(&Self::CHARACTER_CARD_LIST())
            .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    fn writeCardList(preferences: &mut Preferences, cardIds: Vec<String>) {
        let encoded = serde_json::to_string(&cardIds).expect("card list must serialize");
        preferences.set(&Self::CHARACTER_CARD_LIST(), encoded);
    }

    #[allow(non_snake_case)]
    fn removeDeletedTagReferencesFromCharacterCards(
        &self,
    ) -> Result<(), PreferencesDataStoreError> {
        let validTagIds = self
            .tagManager
            .getAllTags()?
            .into_iter()
            .map(|tag| tag.id)
            .collect::<Vec<_>>();
        self.dataStore.edit(|preferences| {
            let cardIds = Self::readCardList(preferences);
            for cardId in cardIds {
                let key =
                    stringPreferencesKey(&format!("character_card_{cardId}_attached_tag_ids"));
                let attached = preferences
                    .get(&key)
                    .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok());
                if let Some(attached) = attached {
                    let filtered = attached
                        .into_iter()
                        .filter(|tagId| validTagIds.contains(tagId))
                        .collect::<Vec<_>>();
                    preferences.set(
                        &key,
                        serde_json::to_string(&filtered).expect("attached tag ids must serialize"),
                    );
                }
            }
        })
    }

    #[allow(non_snake_case)]
    fn migrateLegacyOtherContentToChat(&self) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            let cardIds = Self::readCardList(preferences);
            for cardId in cardIds {
                let legacyKey =
                    stringPreferencesKey(&format!("character_card_{cardId}_other_content"));
                let chatKey =
                    stringPreferencesKey(&format!("character_card_{cardId}_other_content_chat"));
                let voiceKey =
                    stringPreferencesKey(&format!("character_card_{cardId}_other_content_voice"));
                let legacyValue = preferences.get(&legacyKey).cloned();
                let chatValue = preferences.get(&chatKey).cloned();
                if let Some(legacyValue) = legacyValue {
                    if !legacyValue.trim().is_empty()
                        && chatValue
                            .map(|value| value.trim().is_empty())
                            .unwrap_or(true)
                    {
                        preferences.set(&chatKey, legacyValue);
                    }
                }
                let voiceValue = preferences.get(&voiceKey).cloned();
                if voiceValue
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
                    && cardId == Self::DEFAULT_CHARACTER_CARD_ID
                {
                    preferences.set(
                        &voiceKey,
                        CharacterCardBilingualData::getDefaultOtherContentVoice(false),
                    );
                }
                preferences.remove(&legacyKey);
            }
        })
    }
}

fn readJsonVec(preferences: &Preferences, key: &str) -> Vec<String> {
    preferences
        .get(&stringPreferencesKey(key))
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn remapAttachedTagIds(sourceIds: &[String], idMap: &HashMap<String, String>) -> Vec<String> {
    let mut remapped = Vec::new();
    for sourceId in sourceIds {
        let value = idMap
            .get(sourceId)
            .cloned()
            .unwrap_or_else(|| sourceId.clone());
        if !remapped.contains(&value) {
            remapped.push(value);
        }
    }
    remapped
}

#[allow(non_snake_case)]
fn parseTagTypeName(value: &str) -> TagType {
    match value {
        "TONE" => TagType::TONE,
        "CHARACTER" => TagType::CHARACTER,
        "FUNCTION" => TagType::FUNCTION,
        _ => TagType::CUSTOM,
    }
}

#[allow(non_snake_case)]
fn tagTypeName(tagType: &TagType) -> &'static str {
    match tagType {
        TagType::TONE => "TONE",
        TagType::CHARACTER => "CHARACTER",
        TagType::FUNCTION => "FUNCTION",
        TagType::CUSTOM => "CUSTOM",
    }
}

#[allow(non_snake_case)]
fn labeledBlock(label: String, content: &str) -> String {
    if content.trim().is_empty() {
        String::new()
    } else {
        format!("{label}\n{}", content.trim())
    }
}

#[allow(non_snake_case)]
fn joinNonEmpty(parts: Vec<String>, separator: &str) -> String {
    parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(separator)
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
