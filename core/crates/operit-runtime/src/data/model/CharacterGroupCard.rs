pub struct GroupMemberConfig {
    pub characterCardId: String,
    pub order: i32,
}

pub struct CharacterGroupCard {
    pub id: String,
    pub name: String,
    pub members: Vec<GroupMemberConfig>,
}
