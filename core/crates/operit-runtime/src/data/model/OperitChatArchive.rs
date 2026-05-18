pub struct OperitChatArchive {
    pub chats: Vec<OperitArchivedChat>,
}

pub struct OperitArchivedChat {
    pub id: String,
    pub title: String,
}

pub struct OperitArchivedMessage {
    pub id: String,
    pub content: String,
}

pub struct OperitArchivedMessageVariant {
    pub id: String,
    pub content: String,
}
