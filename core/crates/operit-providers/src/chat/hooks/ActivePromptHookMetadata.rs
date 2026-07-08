use serde_json::{Map, Value};

pub const ACTIVE_PROMPT_TYPE_CHARACTER_CARD: &str = "character_card";
pub const ACTIVE_PROMPT_TYPE_CHARACTER_GROUP: &str = "character_group";

#[derive(Clone, Debug, Default)]
pub struct ActivePromptHookMetadata {
    pub active_prompt_type: Option<String>,
    pub active_prompt_id: Option<String>,
    pub active_prompt_name: Option<String>,
    pub chat_id: Option<String>,
    pub role_card_id: Option<String>,
}

impl ActivePromptHookMetadata {
    pub fn to_value_map(&self) -> Map<String, Value> {
        let mut map = Map::new();
        if let Some(value) = &self.active_prompt_type {
            map.insert("type".to_string(), Value::String(value.clone()));
        }
        if let Some(value) = &self.active_prompt_id {
            map.insert("id".to_string(), Value::String(value.clone()));
        }
        if let Some(value) = &self.active_prompt_name {
            map.insert("name".to_string(), Value::String(value.clone()));
        }
        if let Some(value) = &self.chat_id {
            map.insert("chatId".to_string(), Value::String(value.clone()));
        }
        if let Some(value) = &self.role_card_id {
            map.insert("roleCardId".to_string(), Value::String(value.clone()));
        }
        map
    }
}

pub fn build_active_prompt_hook_metadata(
    chat_id: Option<&str>,
    role_card_id: Option<&str>,
) -> ActivePromptHookMetadata {
    let role_card_id = role_card_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let chat_id = chat_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let active_prompt_type = if role_card_id.is_some() {
        Some(ACTIVE_PROMPT_TYPE_CHARACTER_CARD.to_string())
    } else {
        None
    };

    ActivePromptHookMetadata {
        active_prompt_type,
        active_prompt_id: role_card_id.clone(),
        active_prompt_name: None,
        chat_id,
        role_card_id,
    }
}
