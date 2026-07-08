#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterCardChatStats {
    pub characterCardName: Option<String>,
    pub chatCount: i32,
    pub messageCount: i32,
}
