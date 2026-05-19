use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActivePrompt {
    CharacterCard { id: String },
    CharacterGroup { id: String },
}
