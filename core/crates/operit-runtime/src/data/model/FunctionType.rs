use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionType {
    CHAT,
    SUMMARY,
    MEMORY,
    UI_CONTROLLER,
    TRANSLATION,
    GREP,
    ROLE_RESPONSE_PLANNER,
    IMAGE_RECOGNITION,
    AUDIO_RECOGNITION,
    VIDEO_RECOGNITION,
}
