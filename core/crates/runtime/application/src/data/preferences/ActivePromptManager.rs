use operit_store::PreferencesDataStore::{
    combine2, CoroutineScope, PreferencesDataStoreError, SharingStarted, StateFlow,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use operit_model::ActivePrompt::ActivePrompt;

#[derive(Clone)]
pub struct ActivePromptManager {
    characterCardManager: CharacterCardManager,
    characterGroupCardManager: CharacterGroupCardManager,
}

impl ActivePromptManager {
    /// Creates an active prompt manager over explicit runtime store paths.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            characterCardManager: CharacterCardManager::new(paths.clone()),
            characterGroupCardManager: CharacterGroupCardManager::new(paths),
        }
    }

    /// Creates an active prompt manager over the default runtime store paths.
    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    /// Returns the active prompt state derived from selected group and character card.
    #[allow(non_snake_case)]
    pub fn activePromptFlow(&self) -> StateFlow<ActivePrompt> {
        let cardIdFlow = self.characterCardManager.observeActiveCharacterCardId();
        let groupIdFlow = self
            .characterGroupCardManager
            .observeActiveCharacterGroupId();
        let cardIdState = cardIdFlow.stateIn(
            CoroutineScope,
            SharingStarted::Lazily,
            cardIdFlow
                .first()
                .expect("CharacterCardManager.observeActiveCharacterCardId must succeed"),
        );
        let groupIdState = groupIdFlow.stateIn(
            CoroutineScope,
            SharingStarted::Lazily,
            groupIdFlow
                .first()
                .expect("CharacterGroupCardManager.observeActiveCharacterGroupId must succeed"),
        );
        combine2(&groupIdState, &cardIdState, |groupId, cardId| {
            if let Some(groupId) = groupId {
                return ActivePrompt::CharacterGroup { id: groupId };
            }
            match cardId
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            {
                Some(id) => ActivePrompt::CharacterCard { id },
                None => ActivePrompt::CharacterCard {
                    id: CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string(),
                },
            }
        })
    }

    /// Reads the current active prompt snapshot.
    #[allow(non_snake_case)]
    pub fn getActivePrompt(&self) -> Result<ActivePrompt, PreferencesDataStoreError> {
        Ok(self.activePromptFlow().value())
    }

    /// Stores the active prompt and clears the opposite prompt target.
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
                self.characterGroupCardManager
                    .setActiveCharacterGroupCard(None)
            }
        }
    }

    /// Activates a character group or card based on chat binding metadata.
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
            if let Some(targetCard) = self
                .characterCardManager
                .findCharacterCardByName(&cardName)?
            {
                return self.setActivePrompt(ActivePrompt::CharacterCard { id: targetCard.id });
            }
        }

        self.setActivePrompt(ActivePrompt::CharacterCard {
            id: CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string(),
        })
    }

    /// Returns the character card id that should be used for the next send operation.
    #[allow(non_snake_case)]
    pub fn resolveActiveCardIdForSend(&self) -> Result<String, PreferencesDataStoreError> {
        match self.getActivePrompt()? {
            ActivePrompt::CharacterCard { id } => Ok(id),
            ActivePrompt::CharacterGroup { .. } => {
                Ok(CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string())
            }
        }
    }
}
