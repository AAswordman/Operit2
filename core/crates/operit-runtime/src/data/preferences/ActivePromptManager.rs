use operit_store::PreferencesDataStore::{Flow, PreferencesDataStoreError};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use crate::data::model::ActivePrompt::ActivePrompt;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;

#[derive(Clone)]
pub struct ActivePromptManager {
    characterCardManager: CharacterCardManager,
    characterGroupCardManager: CharacterGroupCardManager,
}

impl ActivePromptManager {
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            characterCardManager: CharacterCardManager::new(paths.clone()),
            characterGroupCardManager: CharacterGroupCardManager::new(paths),
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    #[allow(non_snake_case)]
    pub fn activePromptFlow(&self) -> Flow<ActivePrompt> {
        let cardManager = self.characterCardManager.clone();
        let groupManager = self.characterGroupCardManager.clone();
        groupManager.observeActiveCharacterGroupId().map(move |groupId| {
            if let Some(groupId) = groupId {
                return ActivePrompt::CharacterGroup { id: groupId };
            }
            let cardId = cardManager
                .observeActiveCharacterCardId()
                .first()
                .ok()
                .flatten()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string());
            ActivePrompt::CharacterCard { id: cardId }
        })
    }

    #[allow(non_snake_case)]
    pub fn getActivePrompt(&self) -> Result<ActivePrompt, PreferencesDataStoreError> {
        self.activePromptFlow().first()
    }

    #[allow(non_snake_case)]
    pub fn setActivePrompt(&self, prompt: ActivePrompt) -> Result<(), PreferencesDataStoreError> {
        match prompt {
            ActivePrompt::CharacterGroup { id } => {
                self.characterGroupCardManager
                    .setActiveCharacterGroupCard(Some(id))?;
                self.characterCardManager.clearActiveCharacterCard()
            }
            ActivePrompt::CharacterCard { id } => {
                self.characterCardManager.setActiveCharacterCard(&id)?;
                self.characterGroupCardManager.setActiveCharacterGroupCard(None)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn activateForChatBinding(
        &self,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedGroupId = characterGroupId
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if let Some(groupId) = normalizedGroupId {
            return self.setActivePrompt(ActivePrompt::CharacterGroup { id: groupId });
        }

        let normalizedCardName = characterCardName
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if let Some(cardName) = normalizedCardName {
            if let Some(targetCard) = self.characterCardManager.findCharacterCardByName(&cardName)? {
                return self.setActivePrompt(ActivePrompt::CharacterCard { id: targetCard.id });
            }
        }

        self.setActivePrompt(ActivePrompt::CharacterCard {
            id: CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string(),
        })
    }

    #[allow(non_snake_case)]
    pub fn resolveActiveCardIdForSend(&self) -> Result<String, PreferencesDataStoreError> {
        match self.getActivePrompt()? {
            ActivePrompt::CharacterCard { id } => Ok(id),
            ActivePrompt::CharacterGroup { .. } => Ok(CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string()),
        }
    }
}
