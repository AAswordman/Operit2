use std::collections::HashMap;

use operit_host_api::WebVisitLinkData;
use serde::Serialize;

pub enum ToolResultData {
    BooleanResultData(BooleanResultData),
    StringResultData(StringResultData),
    IntResultData(IntResultData),
    BinaryResultData(BinaryResultData),
}

pub struct BooleanResultData {
    pub value: bool,
}

pub struct StringResultData {
    pub value: String,
}

pub struct IntResultData {
    pub value: i32,
}

pub struct BinaryResultData {
    pub value: Vec<u8>,
}

#[derive(Clone, Serialize)]
pub struct ChatServiceStartResultData {
    pub isConnected: bool,
    pub connectionTime: i64,
}

#[derive(Clone, Serialize)]
pub struct ChatCreationResultData {
    pub chatId: String,
    pub createdAt: i64,
}

#[derive(Clone, Serialize)]
pub struct ChatInfo {
    pub id: String,
    pub title: String,
    pub messageCount: i32,
    pub createdAt: String,
    pub updatedAt: String,
    pub isCurrent: bool,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub characterCardName: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct ChatListResultData {
    pub totalCount: usize,
    pub currentChatId: Option<String>,
    pub chats: Vec<ChatInfo>,
}

#[derive(Clone, Serialize)]
pub struct ChatFindResultData {
    pub matchedCount: usize,
    pub chat: Option<ChatInfo>,
}

#[derive(Clone, Serialize)]
pub struct AgentStatusResultData {
    pub chatId: String,
    pub state: String,
    pub message: Option<String>,
    pub isIdle: bool,
    pub isProcessing: bool,
}

#[derive(Clone, Serialize)]
pub struct ChatSwitchResultData {
    pub chatId: String,
    pub chatTitle: String,
    pub switchedAt: i64,
}

#[derive(Clone, Serialize)]
pub struct ChatTitleUpdateResultData {
    pub chatId: String,
    pub title: String,
    pub updatedAt: i64,
}

#[derive(Clone, Serialize)]
pub struct ChatDeleteResultData {
    pub chatId: String,
    pub deletedAt: i64,
}

#[derive(Clone, Serialize)]
pub struct MessageSendResultData {
    pub chatId: String,
    pub message: String,
    pub aiResponse: Option<String>,
    pub receivedAt: Option<i64>,
    pub sentAt: i64,
}

#[derive(Clone, Serialize)]
pub struct ChatMessageInfo {
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub roleName: String,
    pub provider: String,
    pub modelName: String,
}

#[derive(Clone, Serialize)]
pub struct ChatMessagesResultData {
    pub chatId: String,
    pub order: String,
    pub limit: i32,
    pub messages: Vec<ChatMessageInfo>,
}

#[derive(Clone, Serialize)]
pub struct CharacterCardInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub isDefault: bool,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Serialize)]
pub struct CharacterCardListResultData {
    pub totalCount: usize,
    pub cards: Vec<CharacterCardInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisitWebResultData {
    pub url: String,
    pub title: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub links: Vec<LinkData>,
    pub imageLinks: Vec<String>,
    pub visitKey: Option<String>,
    pub contentSavedTo: Option<String>,
    pub contentTruncated: bool,
    pub originalContentLength: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkData {
    pub url: String,
    pub text: String,
}

impl From<WebVisitLinkData> for LinkData {
    fn from(value: WebVisitLinkData) -> Self {
        Self {
            url: value.url,
            text: value.text,
        }
    }
}
