use crate::api::chat::llmprovider::AIService::SharedAiResponseStream;
use crate::api::chat::EnhancedAIService::EnhancedAIService;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::model::AttachmentInfo::AttachmentInfo;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatTurnOptions::ChatTurnOptions;
use crate::data::model::InputProcessingState::InputProcessingState;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use crate::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use crate::services::core::MessageProcessingDelegate::{MessageProcessingDelegate, TextFieldValue};
use crate::services::core::TokenStatisticsDelegate::TokenStatisticsDelegate;
use crate::ui::features::chat::webview::workspace::WorkspaceBackupManager::{
    WorkspaceBackupManager, WorkspaceFileChange,
};
use crate::util::MarkdownRenderStream::{MarkdownRenderEventStream, MarkdownStreamEvent};
use operit_store::PreferencesDataStore::StateFlow;
pub trait ChatServiceUiBridge {}

pub struct EmptyChatServiceUiBridge;

impl ChatServiceUiBridge for EmptyChatServiceUiBridge {}

pub struct ChatServiceCore {
    pub selectionMode: ChatSelectionMode,
    pub enhancedAiService: Option<EnhancedAIService>,
    pub messageProcessingDelegate: MessageProcessingDelegate,
    pub chatHistoryDelegate: ChatHistoryDelegate,
    pub messageCoordinationDelegate: Option<MessageCoordinationDelegate>,
    pub initialized: bool,
    pub onEnhancedAiServiceReady: Option<fn(&EnhancedAIService)>,
    pub additionalOnTurnComplete: Option<fn(Option<String>, i32, i32, i32)>,
    pub uiBridge: EmptyChatServiceUiBridge,
}

impl ChatServiceCore {
    pub fn new(selectionMode: ChatSelectionMode) -> Self {
        let mut core = Self {
            selectionMode: selectionMode.clone(),
            enhancedAiService: None,
            messageProcessingDelegate: MessageProcessingDelegate::default(),
            chatHistoryDelegate: ChatHistoryDelegate::new(selectionMode),
            messageCoordinationDelegate: None,
            initialized: false,
            onEnhancedAiServiceReady: None,
            additionalOnTurnComplete: None,
            uiBridge: EmptyChatServiceUiBridge,
        };
        core.initializeDelegates();
        core
    }

    fn initializeDelegates(&mut self) {
        self.chatHistoryDelegate = ChatHistoryDelegate::new(self.selectionMode.clone());
        self.chatHistoryDelegate.initialize();
        self.messageProcessingDelegate = MessageProcessingDelegate::default();
        let messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
        self.messageCoordinationDelegate = Some(MessageCoordinationDelegate::new(
            self.chatHistoryDelegate.clone_for_core(),
            messageProcessingDelegate,
        ));
        self.syncTokenStatisticsForCurrentChat();
        self.initialized = true;
    }

    #[allow(non_snake_case)]
    fn syncTokenStatisticsForCurrentChat(&mut self) {
        let chatId = self.chatHistoryDelegate.currentChatId.clone();
        if let Some(delegate) = self.messageCoordinationDelegate.as_mut() {
            delegate
                .tokenStatisticsDelegate
                .setActiveChatId(chatId.clone());
            if let Some(chatId) = chatId {
                if let Some(chat) = self
                    .chatHistoryDelegate
                    .chatHistoriesFlow()
                    .value()
                    .into_iter()
                    .find(|chat| chat.id == chatId)
                {
                    delegate.tokenStatisticsDelegate.setTokenCounts(
                        Some(chat.id),
                        chat.inputTokens,
                        chat.outputTokens,
                        chat.currentWindowSize,
                    );
                }
            }
        }
    }

    pub async fn sendUserMessage(
        &mut self,
        promptFunctionType: PromptFunctionType,
        roleCardIdOverride: Option<String>,
        chatIdOverride: Option<String>,
        messageTextOverride: Option<String>,
        proxySenderNameOverride: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        attachments: Vec<AttachmentInfo>,
        replyToMessage: Option<ChatMessage>,
        turnOptions: ChatTurnOptions,
    ) {
        if let (Some(service), Some(delegate)) = (
            self.enhancedAiService.as_mut(),
            self.messageCoordinationDelegate.as_mut(),
        ) {
            delegate.chatHistoryDelegate = self.chatHistoryDelegate.clone_for_core();
            delegate.messageProcessingDelegate = self.messageProcessingDelegate.clone_for_core();
            delegate
                .sendUserMessage(
                    service,
                    promptFunctionType,
                    roleCardIdOverride,
                    chatIdOverride,
                    messageTextOverride,
                    proxySenderNameOverride,
                    chatModelConfigIdOverride,
                    chatModelIndexOverride,
                    attachments,
                    replyToMessage,
                    turnOptions,
                )
                .await;
            self.chatHistoryDelegate = delegate.chatHistoryDelegate.clone_for_core();
            self.messageProcessingDelegate = delegate.messageProcessingDelegate.clone_for_core();
        }
    }

    pub fn cancelCurrentMessage(&mut self) {
        if let Some(chatId) = self.chatHistoryDelegate.currentChatId.clone() {
            self.messageProcessingDelegate.cancelMessage(chatId);
        }
    }

    pub fn cancelMessage(&mut self, chatId: String) {
        self.messageProcessingDelegate.cancelMessage(chatId);
    }

    pub fn updateUserMessage(&mut self, message: String) {
        self.messageProcessingDelegate.updateUserMessage(message);
    }

    pub fn getResponseStream(&self, chatId: String) -> Option<SharedAiResponseStream> {
        self.messageProcessingDelegate.getResponseStream(chatId)
    }

    pub fn splitMarkdownContent(&self, content: String) -> Vec<MarkdownStreamEvent> {
        MarkdownRenderEventStream::fromContent(content)
    }

    pub fn createNewChat(
        &mut self,
        characterCardName: Option<String>,
        group: Option<String>,
        inheritGroupFromCurrent: bool,
        setAsCurrentChat: bool,
        characterGroupId: Option<String>,
    ) {
        self.chatHistoryDelegate.createNewChat(
            characterCardName,
            characterGroupId,
            group,
            inheritGroupFromCurrent,
            setAsCurrentChat,
            None,
        );
        self.syncTokenStatisticsForCurrentChat();
    }

    pub fn switchChat(&mut self, chatId: String) {
        self.chatHistoryDelegate.switchChat(chatId, true);
        self.syncTokenStatisticsForCurrentChat();
    }

    pub fn switchChatLocal(&mut self, chatId: String) {
        self.chatHistoryDelegate.switchChat(chatId, false);
        self.syncTokenStatisticsForCurrentChat();
    }

    pub fn syncCurrentChatIdToGlobal(&mut self) {}

    pub fn deleteChatHistory(&mut self, chatId: String) {
        self.chatHistoryDelegate.deleteChatHistory(chatId);
    }

    pub fn deleteMessage(&mut self, index: usize) {
        self.chatHistoryDelegate.deleteMessage(index);
    }

    pub fn clearCurrentChat(&mut self) {
        self.chatHistoryDelegate.clearCurrentChat();
    }

    pub fn updateChatTitle(&mut self, chatId: String, title: String) {
        self.chatHistoryDelegate.updateChatTitle(chatId, title);
    }

    #[allow(non_snake_case)]
    pub fn bindChatToWorkspace(
        &mut self,
        chatId: String,
        workspace: String,
        workspaceEnv: Option<String>,
    ) {
        self.chatHistoryDelegate
            .bindChatToWorkspace(chatId, workspace, workspaceEnv);
    }

    #[allow(non_snake_case)]
    pub fn unbindChatFromWorkspace(&mut self, chatId: String) {
        self.chatHistoryDelegate.unbindChatFromWorkspace(chatId);
    }

    #[allow(non_snake_case)]
    pub fn renameWorkspaceAndChat(
        &mut self,
        chatId: String,
        newWorkspace: String,
        newTitle: String,
    ) {
        self.chatHistoryDelegate
            .renameWorkspaceAndChat(chatId, newWorkspace, newTitle);
    }

    #[allow(non_snake_case)]
    pub fn previewWorkspaceChangesForMessage(&mut self, index: usize) -> Vec<WorkspaceFileChange> {
        let Some((chatId, workspacePath, workspaceEnv, rewindTimestamp)) =
            self.resolveWorkspaceRewindTarget(index)
        else {
            return Vec::new();
        };
        WorkspaceBackupManager::getInstance(AIToolHandler::default().getContext())
            .previewChangesForRewind(workspacePath, workspaceEnv, rewindTimestamp, Some(chatId))
    }

    #[allow(non_snake_case)]
    pub fn rewindWorkspaceForMessage(&mut self, index: usize) -> bool {
        let Some((chatId, workspacePath, workspaceEnv, rewindTimestamp)) =
            self.resolveWorkspaceRewindTarget(index)
        else {
            return false;
        };
        WorkspaceBackupManager::getInstance(AIToolHandler::default().getContext()).syncState(
            workspacePath,
            rewindTimestamp,
            workspaceEnv,
            Some(chatId),
        );
        true
    }

    #[allow(non_snake_case)]
    pub fn rollbackToMessage(&mut self, index: usize) -> bool {
        let Some(targetMessage) = self.chatHistoryDelegate.chatHistory.get(index).cloned() else {
            return false;
        };
        if targetMessage.sender != "user" {
            return false;
        }
        self.rewindWorkspaceForMessage(index);
        self.chatHistoryDelegate
            .truncateChatHistory(Some(targetMessage.timestamp));
        self.messageProcessingDelegate
            .updateUserMessage(stripXmlLikeTags(&targetMessage.content));
        true
    }

    #[allow(non_snake_case)]
    pub async fn rewindAndResendMessage(&mut self, index: usize, editedContent: String) -> bool {
        let Some(targetMessage) = self.chatHistoryDelegate.chatHistory.get(index).cloned() else {
            return false;
        };
        if targetMessage.sender != "user" {
            return false;
        }
        self.rewindWorkspaceForMessage(index);
        self.chatHistoryDelegate
            .truncateChatHistory(Some(targetMessage.timestamp));
        self.messageProcessingDelegate
            .updateUserMessage(editedContent);
        self.sendUserMessage(
            PromptFunctionType::CHAT,
            None,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            ChatTurnOptions::default(),
        )
        .await;
        true
    }

    #[allow(non_snake_case)]
    fn resolveWorkspaceRewindTarget(
        &self,
        index: usize,
    ) -> Option<(String, String, Option<String>, i64)> {
        let chatId = self.chatHistoryDelegate.currentChatId.clone()?;
        if index >= self.chatHistoryDelegate.chatHistory.len() {
            return None;
        }
        let rewindTimestamp = if index > 0 {
            self.chatHistoryDelegate.chatHistory[index - 1].timestamp
        } else {
            0
        };
        let currentChat = self
            .chatHistoryDelegate
            .chatHistories
            .iter()
            .find(|history| history.id == chatId)?;
        let workspacePath = currentChat
            .workspace
            .clone()
            .filter(|value| !value.trim().is_empty())?;
        Some((
            chatId,
            workspacePath,
            currentChat.workspaceEnv.clone(),
            rewindTimestamp,
        ))
    }

    pub fn resetTokenStatistics(&mut self) {}

    pub fn updateCumulativeStatistics(&mut self) {}

    pub fn handleAttachment(&mut self, _filePath: String) {}

    pub fn removeAttachment(&mut self, _filePath: String) {}

    pub fn clearAttachments(&mut self) {}

    pub fn userMessage(&self) -> &TextFieldValue {
        &self.messageProcessingDelegate.userMessage
    }

    pub fn userMessageFlow(&self) -> StateFlow<TextFieldValue> {
        self.messageProcessingDelegate.userMessageFlow()
    }

    pub fn isLoading(&self) -> bool {
        self.messageProcessingDelegate.isLoading
    }

    pub fn isLoadingFlow(&self) -> StateFlow<bool> {
        self.messageProcessingDelegate.isLoadingFlow()
    }

    pub fn activeStreamingChatIds(&self) -> Vec<String> {
        self.messageProcessingDelegate
            .activeStreamingChatIds
            .iter()
            .cloned()
            .collect()
    }

    pub fn activeStreamingChatIdsFlow(&self) -> StateFlow<std::collections::HashSet<String>> {
        self.messageProcessingDelegate.activeStreamingChatIdsFlow()
    }

    pub fn inputProcessingStateByChatId(
        &self,
    ) -> &std::collections::HashMap<String, InputProcessingState> {
        &self.messageProcessingDelegate.inputProcessingStateByChatId
    }

    pub fn inputProcessingStateByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, InputProcessingState>> {
        self.messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
    }

    #[allow(non_snake_case)]
    pub fn currentChatInputProcessingState(&self) -> InputProcessingState {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow().value() else {
            return InputProcessingState::Idle;
        };
        match self
            .messageProcessingDelegate
            .inputProcessingStateByChatIdFlow()
            .value()
            .get(&chatId)
            .cloned()
        {
            Some(state) => state,
            None => InputProcessingState::Idle,
        }
    }

    #[allow(non_snake_case)]
    pub fn currentChatIsLoading(&self) -> bool {
        let Some(chatId) = self.chatHistoryDelegate.currentChatIdFlow().value() else {
            return false;
        };
        self.messageProcessingDelegate
            .activeStreamingChatIdsFlow()
            .value()
            .contains(&chatId)
    }

    pub fn currentTurnToolInvocationCountByChatId(
        &self,
    ) -> &std::collections::HashMap<String, i32> {
        &self
            .messageProcessingDelegate
            .currentTurnToolInvocationCountByChatId
    }

    pub fn currentTurnToolInvocationCountByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, i32>> {
        self.messageProcessingDelegate
            .currentTurnToolInvocationCountByChatIdFlow()
    }

    pub fn chatHistory(&self) -> &Vec<ChatMessage> {
        &self.chatHistoryDelegate.chatHistory
    }

    #[allow(non_snake_case)]
    pub fn chatHistoryFlow(&self) -> StateFlow<Vec<ChatMessage>> {
        self.chatHistoryDelegate.chatHistoryFlow()
    }

    pub fn currentChatId(&self) -> &Option<String> {
        &self.chatHistoryDelegate.currentChatId
    }

    #[allow(non_snake_case)]
    pub fn currentChatIdFlow(&self) -> StateFlow<Option<String>> {
        self.chatHistoryDelegate.currentChatIdFlow()
    }

    pub fn chatHistories(&self) -> &Vec<crate::data::model::ChatHistory::ChatHistory> {
        &self.chatHistoryDelegate.chatHistories
    }

    #[allow(non_snake_case)]
    pub fn chatHistoriesFlow(
        &self,
    ) -> StateFlow<Vec<crate::data::model::ChatHistory::ChatHistory>> {
        self.chatHistoryDelegate.chatHistoriesFlow()
    }

    pub fn showChatHistorySelector(&self) -> bool {
        self.chatHistoryDelegate.showChatHistorySelector
    }

    pub fn attachments(&self) -> Vec<AttachmentInfo> {
        Vec::new()
    }

    pub fn getChatHistoryDelegate(&mut self) -> &mut ChatHistoryDelegate {
        &mut self.chatHistoryDelegate
    }

    pub fn getMessageProcessingDelegate(&mut self) -> &mut MessageProcessingDelegate {
        &mut self.messageProcessingDelegate
    }

    pub fn getMessageCoordinationDelegate(&mut self) -> Option<&mut MessageCoordinationDelegate> {
        self.messageCoordinationDelegate.as_mut()
    }

    #[allow(non_snake_case)]
    pub fn getTokenStatisticsDelegate(&self) -> Option<&TokenStatisticsDelegate> {
        self.messageCoordinationDelegate
            .as_ref()
            .map(|delegate| &delegate.tokenStatisticsDelegate)
    }

    #[allow(non_snake_case)]
    pub fn currentWindowSizeFlow(&self) -> StateFlow<i32> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .currentWindowSizeFlow()
    }

    #[allow(non_snake_case)]
    pub fn inputTokenCountFlow(&self) -> StateFlow<i32> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .cumulativeInputTokensFlow()
    }

    #[allow(non_snake_case)]
    pub fn outputTokenCountFlow(&self) -> StateFlow<i32> {
        self.getTokenStatisticsDelegate()
            .expect("TokenStatisticsDelegate must be initialized")
            .cumulativeOutputTokensFlow()
    }

    pub fn getEnhancedAiService(&self) -> Option<&EnhancedAIService> {
        self.enhancedAiService.as_ref()
    }

    pub fn isInitialized(&self) -> bool {
        self.initialized
    }

    pub fn setOnEnhancedAiServiceReady(&mut self, callback: fn(&EnhancedAIService)) {
        self.onEnhancedAiServiceReady = Some(callback);
    }

    pub fn setAdditionalOnTurnComplete(
        &mut self,
        callback: Option<fn(Option<String>, i32, i32, i32)>,
    ) {
        self.additionalOnTurnComplete = callback;
    }

    pub fn setUiBridge(&mut self, uiBridge: EmptyChatServiceUiBridge) {
        self.uiBridge = uiBridge;
    }

    pub fn setSpeakMessageHandler(&mut self, handler: fn(String, bool)) {
        self.messageProcessingDelegate
            .setSpeakMessageHandler(handler);
    }

    pub fn reloadChatMessagesSmart(&mut self, chatId: String) {
        self.chatHistoryDelegate.reloadChatMessagesSmart(chatId);
    }

    pub fn loadOlderMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.loadOlderMessagesForCurrentChat();
    }

    pub fn loadNewerMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.loadNewerMessagesForCurrentChat();
    }

    pub fn showLatestMessagesForCurrentChat(&mut self) {
        self.chatHistoryDelegate.showLatestMessagesForCurrentChat();
    }
}

impl Default for ChatServiceCore {
    fn default() -> Self {
        Self::new(ChatSelectionMode::FOLLOW_GLOBAL)
    }
}

#[allow(non_snake_case)]
fn stripXmlLikeTags(text: &str) -> String {
    let mut value = text.to_string();
    for _ in 0..5 {
        let updated = removePairedXmlLikeTags(&value);
        if updated == value {
            break;
        }
        value = updated;
    }
    value = removeSelfClosingXmlLikeTags(&value);
    removeRemainingXmlLikeTags(&value).trim().to_string()
}

#[allow(non_snake_case)]
fn removePairedXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        if let Some(tagName) = parseOpeningXmlLikeTag(text, openStart, openEnd) {
            if let Some(closeEnd) = findClosingXmlLikeTagEnd(text, openEnd + 1, tagName) {
                result.push_str(&text[cursor..openStart]);
                cursor = closeEnd;
                continue;
            }
        }

        result.push_str(&text[cursor..openStart + 1]);
        cursor = openStart + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn removeSelfClosingXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        if parseSelfClosingXmlLikeTag(text, openStart, openEnd) {
            result.push_str(&text[cursor..openStart]);
            cursor = openEnd + 1;
            continue;
        }

        result.push_str(&text[cursor..openStart + 1]);
        cursor = openStart + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn removeRemainingXmlLikeTags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(openRelativeStart) = text[cursor..].find('<') {
        let openStart = cursor + openRelativeStart;
        let Some(openEnd) = text[openStart..].find('>').map(|offset| openStart + offset) else {
            break;
        };

        result.push_str(&text[cursor..openStart]);
        cursor = openEnd + 1;
    }

    result.push_str(&text[cursor..]);
    result
}

#[allow(non_snake_case)]
fn parseOpeningXmlLikeTag(text: &str, openStart: usize, openEnd: usize) -> Option<&str> {
    let body = text.get(openStart + 1..openEnd)?;
    if body.starts_with('/') || body.trim_end().ends_with('/') {
        return None;
    }
    parseXmlLikeTagName(body)
}

#[allow(non_snake_case)]
fn parseSelfClosingXmlLikeTag(text: &str, openStart: usize, openEnd: usize) -> bool {
    let Some(body) = text.get(openStart + 1..openEnd) else {
        return false;
    };
    if body.starts_with('/') || !body.trim_end().ends_with('/') {
        return false;
    }
    parseXmlLikeTagName(body).is_some()
}

#[allow(non_snake_case)]
fn parseXmlLikeTagName(body: &str) -> Option<&str> {
    let bytes = body.as_bytes();
    let first = *bytes.first()?;
    if !isXmlLikeTagNameStart(first) {
        return None;
    }

    let mut end = 1;
    while end < bytes.len() && isXmlLikeTagNameChar(bytes[end]) {
        end += 1;
    }

    if end < bytes.len() {
        let rest = &body[end..];
        if !rest
            .chars()
            .next()
            .is_some_and(|value| value.is_whitespace())
        {
            return None;
        }
    }

    Some(&body[..end])
}

#[allow(non_snake_case)]
fn findClosingXmlLikeTagEnd(text: &str, from: usize, tagName: &str) -> Option<usize> {
    let mut searchStart = 0;

    while let Some(relativeStart) = text[from + searchStart..].find("</") {
        let closeStart = from + searchStart + relativeStart;
        if let Some(closeEnd) = text[closeStart..]
            .find('>')
            .map(|offset| closeStart + offset)
        {
            let body = &text[closeStart + 2..closeEnd];
            if body.eq_ignore_ascii_case(tagName) {
                return Some(closeEnd + 1);
            }
        }
        searchStart += relativeStart + 2;
    }

    None
}

#[allow(non_snake_case)]
fn isXmlLikeTagNameStart(value: u8) -> bool {
    value.is_ascii_alphabetic()
}

#[allow(non_snake_case)]
fn isXmlLikeTagNameChar(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b':' | b'_' | b'-')
}
