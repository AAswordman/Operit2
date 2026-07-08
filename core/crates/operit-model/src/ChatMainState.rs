use serde::{Deserialize, Serialize};

use super::ChatMessage::ChatMessage;
use super::InputProcessingState::InputProcessingState;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMainState {
    pub currentChatId: Option<String>,
    pub currentChatTitle: String,
    pub currentModelName: String,
    pub currentCharacterCardName: Option<String>,
    pub currentCharacterCardAvatarUri: Option<String>,
    pub currentWorkspacePath: Option<String>,
    pub activeCharacterCardName: Option<String>,
    pub isLoading: bool,
    pub inputProcessingState: InputProcessingState,
    pub messages: Vec<ChatMessage>,
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
}
