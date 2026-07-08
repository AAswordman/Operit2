#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterGroupChatStats {
    pub characterGroupId: Option<String>,
    pub chatCount: i32,
    pub messageCount: i32,
}
