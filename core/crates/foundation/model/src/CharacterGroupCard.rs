use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GroupMemberConfig {
    pub characterCardId: String,
    pub orderIndex: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CharacterGroupCard {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: Vec<GroupMemberConfig>,
    pub createdAt: i64,
    pub updatedAt: i64,
}

impl CharacterGroupCard {
    pub fn new(id: String, name: String) -> Self {
        let now = currentTimeMillis();
        Self {
            id,
            name,
            description: String::new(),
            members: Vec::new(),
            createdAt: now,
            updatedAt: now,
        }
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
