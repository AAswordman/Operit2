use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use uuid::Uuid;

use crate::data::model::CharacterCard::{
    CharacterCard, CharacterCardChatModelBindingMode, CharacterCardMemoryProfileBindingMode,
    CharacterCardToolAccessConfig,
};
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::preferences::CharacterCardBilingualData::CharacterCardBilingualData;
use crate::data::preferences::PromptTagManager::PromptTagManager;

#[derive(Clone)]
pub struct CharacterCardManager {
    dataStore: PreferencesDataStore,
    tagManager: PromptTagManager,
}

impl CharacterCardManager {
    #[allow(non_snake_case)]
    pub const DEFAULT_CHARACTER_CARD_ID: &str = "default_character";
    #[allow(non_snake_case)]
    pub const DEFAULT_CHARACTER_NAME: &str = "Operit";

    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(paths.root_dir().join("character_cards.preferences.json")),
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
        self.dataStore.dataFlow().map(|preferences| Self::readCardList(&preferences))
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
    fn getCharacterCardFromPreferences(&self, preferences: &Preferences, id: &str) -> CharacterCard {
        CharacterCard {
            id: id.to_string(),
            name: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_name")))
                .cloned()
                .unwrap_or_else(|| Self::DEFAULT_CHARACTER_NAME.to_string()),
            description: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_description")))
                .cloned()
                .unwrap_or_default(),
            characterSetting: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_character_setting")))
                .cloned()
                .unwrap_or_default(),
            openingStatement: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_opening_statement")))
                .cloned()
                .unwrap_or_default(),
            otherContentChat: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_other_content_chat")))
                .cloned()
                .unwrap_or_default(),
            otherContentVoice: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_other_content_voice")))
                .cloned()
                .unwrap_or_default(),
            attachedTagIds: readJsonVec(preferences, &format!("character_card_{id}_attached_tag_ids")),
            advancedCustomPrompt: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_advanced_custom_prompt")))
                .cloned()
                .unwrap_or_default(),
            marks: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_marks")))
                .cloned()
                .unwrap_or_default(),
            chatModelBindingMode: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_chat_model_binding_mode")))
                .map(|value| CharacterCardChatModelBindingMode::normalize(Some(value)))
                .unwrap_or_else(|| CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string()),
            chatModelConfigId: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_chat_model_config_id")))
                .cloned(),
            chatModelIndex: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_chat_model_index")))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(0),
            memoryProfileBindingMode: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_memory_profile_binding_mode")))
                .map(|value| CharacterCardMemoryProfileBindingMode::normalize(Some(value)))
                .unwrap_or_else(|| CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL.to_string()),
            memoryProfileId: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_memory_profile_id")))
                .cloned(),
            toolAccessConfig: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_tool_access_config_json")))
                .and_then(|raw| serde_json::from_str::<CharacterCardToolAccessConfig>(raw).ok())
                .unwrap_or_default()
                .normalized(),
            isDefault: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_is_default")))
                .map(|value| value == "true")
                .unwrap_or(id == Self::DEFAULT_CHARACTER_CARD_ID),
            createdAt: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_created_at")))
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_else(currentTimeMillis),
            updatedAt: preferences
                .get(&stringPreferencesKey(&format!("character_card_{id}_updated_at")))
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_else(currentTimeMillis),
        }
    }

    #[allow(non_snake_case)]
    pub fn createCharacterCard(&self, card: CharacterCard) -> Result<String, PreferencesDataStoreError> {
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
    pub fn updateCharacterCard(&self, card: CharacterCard) -> Result<(), PreferencesDataStoreError> {
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
    pub fn findCharacterCardByName(&self, name: &str) -> Result<Option<CharacterCard>, PreferencesDataStoreError> {
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
            if currentList.as_ref().map(|list| list.is_empty()).unwrap_or(true) {
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
        for tagId in characterCard.attachedTagIds.into_iter().chain(additionalTagIds.into_iter()) {
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
    fn writeCard(&self, preferences: &mut Preferences, card: &CharacterCard, id: &str, now: i64) {
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_name")), card.name.clone());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_description")), card.description.clone());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_character_setting")), card.characterSetting.clone());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_opening_statement")), card.openingStatement.clone());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_other_content_chat")), card.otherContentChat.clone());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_other_content_voice")), card.otherContentVoice.clone());
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_attached_tag_ids")),
            serde_json::to_string(&card.attachedTagIds).expect("attached tag ids must serialize"),
        );
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_advanced_custom_prompt")), card.advancedCustomPrompt.clone());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_marks")), card.marks.clone());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_chat_model_binding_mode")), card.chatModelBindingMode.clone());
        if let Some(value) = &card.chatModelConfigId {
            preferences.set(&stringPreferencesKey(&format!("character_card_{id}_chat_model_config_id")), value.clone());
        } else {
            preferences.remove(&stringPreferencesKey(&format!("character_card_{id}_chat_model_config_id")));
        }
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_chat_model_index")), card.chatModelIndex.max(0).to_string());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_memory_profile_binding_mode")), card.memoryProfileBindingMode.clone());
        if let Some(value) = &card.memoryProfileId {
            preferences.set(&stringPreferencesKey(&format!("character_card_{id}_memory_profile_id")), value.clone());
        } else {
            preferences.remove(&stringPreferencesKey(&format!("character_card_{id}_memory_profile_id")));
        }
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_tool_access_config_json")),
            serde_json::to_string(&card.toolAccessConfig.normalized()).expect("tool access config must serialize"),
        );
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_is_default")), card.isDefault.to_string());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_created_at")), card.createdAt.to_string());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_updated_at")), now.to_string());
    }

    #[allow(non_snake_case)]
    fn setupDefaultCharacterCard(&self, preferences: &mut Preferences, id: &str) {
        let now = currentTimeMillis();
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_name")), Self::DEFAULT_CHARACTER_NAME.to_string());
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_description")),
            CharacterCardBilingualData::getDefaultDescription(false),
        );
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_character_setting")),
            CharacterCardBilingualData::getDefaultCharacterSetting(false),
        );
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_opening_statement")), String::new());
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
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_advanced_custom_prompt")), String::new());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_marks")), String::new());
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_chat_model_binding_mode")),
            CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string(),
        );
        preferences.remove(&stringPreferencesKey(&format!("character_card_{id}_chat_model_config_id")));
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_chat_model_index")), "0".to_string());
        preferences.set(
            &stringPreferencesKey(&format!("character_card_{id}_memory_profile_binding_mode")),
            CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL.to_string(),
        );
        preferences.remove(&stringPreferencesKey(&format!("character_card_{id}_memory_profile_id")));
        preferences.remove(&stringPreferencesKey(&format!("character_card_{id}_tool_access_config_json")));
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_is_default")), true.to_string());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_created_at")), now.to_string());
        preferences.set(&stringPreferencesKey(&format!("character_card_{id}_updated_at")), now.to_string());
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
            preferences.remove(&stringPreferencesKey(&format!("character_card_{id}_{suffix}")));
        }
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
    fn removeDeletedTagReferencesFromCharacterCards(&self) -> Result<(), PreferencesDataStoreError> {
        let validTagIds = self
            .tagManager
            .getAllTags()?
            .into_iter()
            .map(|tag| tag.id)
            .collect::<Vec<_>>();
        self.dataStore.edit(|preferences| {
            let cardIds = Self::readCardList(preferences);
            for cardId in cardIds {
                let key = stringPreferencesKey(&format!("character_card_{cardId}_attached_tag_ids"));
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
                let legacyKey = stringPreferencesKey(&format!("character_card_{cardId}_other_content"));
                let chatKey = stringPreferencesKey(&format!("character_card_{cardId}_other_content_chat"));
                let voiceKey = stringPreferencesKey(&format!("character_card_{cardId}_other_content_voice"));
                let legacyValue = preferences.get(&legacyKey).cloned();
                let chatValue = preferences.get(&chatKey).cloned();
                if let Some(legacyValue) = legacyValue {
                    if !legacyValue.trim().is_empty()
                        && chatValue.map(|value| value.trim().is_empty()).unwrap_or(true)
                    {
                        preferences.set(&chatKey, legacyValue);
                    }
                }
                let voiceValue = preferences.get(&voiceKey).cloned();
                if voiceValue.map(|value| value.trim().is_empty()).unwrap_or(true)
                    && cardId == Self::DEFAULT_CHARACTER_CARD_ID
                {
                    preferences.set(&voiceKey, CharacterCardBilingualData::getDefaultOtherContentVoice(false));
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
fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_millis() as i64
}
