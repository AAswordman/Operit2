use crate::api::chat::EnhancedAIService::EnhancedAIService;
use crate::api::chat::llmprovider::AIService::SharedAiResponseStream;
use crate::data::model::AttachmentInfo::AttachmentInfo;
use crate::data::model::ChatMessage::ChatMessage;
use crate::data::model::ChatTurnOptions::ChatTurnOptions;
use crate::data::model::InputProcessingState::InputProcessingState;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::services::core::ChatHistoryDelegate::{ChatHistoryDelegate, ChatSelectionMode};
use crate::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use crate::services::core::MessageProcessingDelegate::{MessageProcessingDelegate, TextFieldValue};
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
        self.initialized = true;
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
            delegate.sendUserMessage(
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
    }

    pub fn switchChat(&mut self, chatId: String) {
        self.chatHistoryDelegate.switchChat(chatId, true);
    }

    pub fn switchChatLocal(&mut self, chatId: String) {
        self.chatHistoryDelegate.switchChat(chatId, false);
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

    pub fn inputProcessingStateByChatId(&self) -> &std::collections::HashMap<String, InputProcessingState> {
        &self.messageProcessingDelegate.inputProcessingStateByChatId
    }

    pub fn inputProcessingStateByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, InputProcessingState>> {
        self.messageProcessingDelegate.inputProcessingStateByChatIdFlow()
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

    pub fn currentTurnToolInvocationCountByChatId(&self) -> &std::collections::HashMap<String, i32> {
        &self
            .messageProcessingDelegate
            .currentTurnToolInvocationCountByChatId
    }

    pub fn currentTurnToolInvocationCountByChatIdFlow(
        &self,
    ) -> StateFlow<std::collections::HashMap<String, i32>> {
        self.messageProcessingDelegate.currentTurnToolInvocationCountByChatIdFlow()
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
    pub fn chatHistoriesFlow(&self) -> StateFlow<Vec<crate::data::model::ChatHistory::ChatHistory>> {
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
        self.messageProcessingDelegate.setSpeakMessageHandler(handler);
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
