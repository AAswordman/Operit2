use std::collections::HashMap;

use crate::api::chat::enhance::ConversationService::ConversationService;
use crate::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::api::chat::EnhancedAIService::EnhancedAIService;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::services::core::ChatHistoryDelegate::ChatSelectionMode;
use crate::services::ChatServiceCore::ChatServiceCore;

pub struct ChatRuntimeCoreFactory {
    applicationContext: Option<OperitApplicationContext>,
}

impl ChatRuntimeCoreFactory {
    pub fn bootstrap() -> Self {
        Self {
            applicationContext: None,
        }
    }

    pub fn new(applicationContext: OperitApplicationContext) -> Self {
        Self {
            applicationContext: Some(applicationContext),
        }
    }

    pub fn createCore(&self, slot: ChatRuntimeSlot) -> ChatServiceCore {
        let mut core = ChatServiceCore::new(match slot {
            ChatRuntimeSlot::MAIN => ChatSelectionMode::FOLLOW_GLOBAL,
            ChatRuntimeSlot::FLOATING | ChatRuntimeSlot::DETACHED(_) => {
                ChatSelectionMode::LOCAL_ONLY
            }
        });
        if let Some(applicationContext) = &self.applicationContext {
            core.enhancedAiService = Some(EnhancedAIService::newWithToolHandler(
                ConversationService,
                AIToolHandler::getInstance(applicationContext.clone()),
            ));
        }
        core
    }
}

pub struct ChatRuntimeHolder {
    pub cores: HashMap<ChatRuntimeSlot, ChatServiceCore>,
    pub activeConversationCount: i32,
    pub currentSessionToolCount: i32,
    coreFactory: ChatRuntimeCoreFactory,
}

impl ChatRuntimeHolder {
    pub fn new() -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::bootstrap())
    }

    #[allow(non_snake_case)]
    pub fn newWithApplicationContext(applicationContext: OperitApplicationContext) -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::new(applicationContext))
    }

    #[allow(non_snake_case)]
    pub fn newWithFactory(coreFactory: ChatRuntimeCoreFactory) -> Self {
        let mut holder = Self {
            cores: HashMap::new(),
            activeConversationCount: 0,
            currentSessionToolCount: 0,
            coreFactory,
        };
        for slot in [ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING] {
            holder.getCore(slot);
        }
        holder.setupCrossSessionSync();
        holder.observeStats();
        holder
    }

    #[allow(non_snake_case)]
    pub fn getCore(&mut self, slot: ChatRuntimeSlot) -> &mut ChatServiceCore {
        if !self.cores.contains_key(&slot) {
            let core = self.coreFactory.createCore(slot.clone());
            self.cores.insert(slot.clone(), core);
        }
        self.cores
            .get_mut(&slot)
            .expect("ChatRuntimeHolder core must exist after insertion")
    }

    #[allow(non_snake_case)]
    pub fn observeStats(&mut self) {
        let activeConversationCount = self
            .cores
            .values()
            .map(|core| core.activeStreamingChatIds().len() as i32)
            .sum();
        let currentSessionToolCount = self
            .cores
            .values()
            .map(|core| {
                core.activeStreamingChatIds()
                    .iter()
                    .map(|chatId| {
                        core.currentTurnToolInvocationCountByChatId()
                            .get(chatId)
                            .copied()
                            .unwrap_or(0)
                    })
                    .sum::<i32>()
            })
            .sum();
        self.activeConversationCount = activeConversationCount;
        self.currentSessionToolCount = currentSessionToolCount;
    }

    #[allow(non_snake_case)]
    pub fn setupCrossSessionSync(&mut self) {
        self.registerChatSelectionSync(ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING);
        self.registerTurnSync(ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING);
        self.registerTurnSync(ChatRuntimeSlot::FLOATING, ChatRuntimeSlot::MAIN);
    }

    #[allow(non_snake_case)]
    pub fn registerTurnSync(&mut self, _sourceSlot: ChatRuntimeSlot, _targetSlot: ChatRuntimeSlot) {
    }

    #[allow(non_snake_case)]
    pub fn syncMainChatSelectionToFloating(&mut self, chatId: String) {
        if chatId.trim().is_empty() {
            return;
        }
        self.syncChatSelection(ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING, chatId);
    }

    #[allow(non_snake_case)]
    pub fn registerChatSelectionSync(
        &mut self,
        _sourceSlot: ChatRuntimeSlot,
        _targetSlot: ChatRuntimeSlot,
    ) {
    }

    #[allow(non_snake_case)]
    pub fn syncChatSelection(
        &mut self,
        _sourceSlot: ChatRuntimeSlot,
        targetSlot: ChatRuntimeSlot,
        chatId: String,
    ) {
        let targetCore = self.getCore(targetSlot);
        if targetCore.currentChatId().as_ref() == Some(&chatId) {
            return;
        }
        targetCore.switchChatLocal(chatId);
    }
}

impl Default for ChatRuntimeHolder {
    fn default() -> Self {
        Self::new()
    }
}
