use std::collections::HashMap;

use crate::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::services::core::ChatHistoryDelegate::ChatSelectionMode;
use crate::services::ChatServiceCore::ChatServiceCore;

pub struct ChatRuntimeHolder {
    pub cores: HashMap<ChatRuntimeSlot, ChatServiceCore>,
    pub activeConversationCount: i32,
    pub currentSessionToolCount: i32,
}

impl ChatRuntimeHolder {
    pub fn new() -> Self {
        let mut holder = Self {
            cores: HashMap::new(),
            activeConversationCount: 0,
            currentSessionToolCount: 0,
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
        self.cores.entry(slot.clone()).or_insert_with(|| {
            ChatServiceCore::new(match slot {
                ChatRuntimeSlot::MAIN => ChatSelectionMode::FOLLOW_GLOBAL,
                ChatRuntimeSlot::FLOATING => ChatSelectionMode::LOCAL_ONLY,
            })
        })
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
    pub fn registerTurnSync(
        &mut self,
        _sourceSlot: ChatRuntimeSlot,
        _targetSlot: ChatRuntimeSlot,
    ) {
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
