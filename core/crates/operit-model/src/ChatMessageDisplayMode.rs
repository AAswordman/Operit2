use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatMessageDisplayMode {
    NORMAL,
    HIDDEN_PLACEHOLDER,
}
