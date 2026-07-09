use std::collections::HashMap;

use crate::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::services::core::ChatHistoryDelegate::ChatSelectionMode;
use crate::services::ChatServiceCore::ChatServiceCore;
use operit_host_api::HostManager::HostManager;
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_tools::tools::AIToolHandler::AIToolHandler;

/// Builds chat service cores for each runtime slot.
pub struct ChatRuntimeCoreFactory {
    hostManager: Option<HostManager>,
}

impl ChatRuntimeCoreFactory {
    /// Creates a factory used before host capabilities have been installed.
    pub fn bootstrap() -> Self {
        Self { hostManager: None }
    }

    /// Creates a factory that wires chat cores to host-backed tools and services.
    pub fn new(hostManager: HostManager) -> Self {
        Self {
            hostManager: Some(hostManager),
        }
    }

    /// Creates a chat service core configured for the requested slot.
    pub fn createCore(&self, slot: ChatRuntimeSlot) -> ChatServiceCore {
        let mut core = ChatServiceCore::new(match slot {
            ChatRuntimeSlot::MAIN => ChatSelectionMode::FOLLOW_GLOBAL,
            ChatRuntimeSlot::FLOATING | ChatRuntimeSlot::DETACHED(_) => {
                ChatSelectionMode::LOCAL_ONLY
            }
        });
        if let Some(hostManager) = &self.hostManager {
            core.enhancedAiService = Some(EnhancedAIService::newWithToolHandler(
                ConversationService,
                AIToolHandler::getInstance(hostManager.clone()),
            ));
        }
        core
    }
}

/// Keeps the main, floating, and detached chat runtimes in one process-level holder.
pub struct ChatRuntimeHolder {
    pub cores: HashMap<ChatRuntimeSlot, ChatServiceCore>,
    pub activeConversationCount: i32,
    pub currentSessionToolCount: i32,
    coreFactory: ChatRuntimeCoreFactory,
}

impl ChatRuntimeHolder {
    /// Creates a holder using bootstrap cores without host-backed enhanced AI services.
    pub fn new() -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::bootstrap())
    }

    /// Creates a holder that injects host capabilities into newly created cores.
    #[allow(non_snake_case)]
    pub fn newWithHostManager(hostManager: HostManager) -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::new(hostManager))
    }

    /// Creates a holder with a custom core factory and eager main/floating cores.
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

    /// Returns the core for a slot, creating it from the factory when first used.
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

    /// Refreshes aggregate active-conversation and tool-invocation counters.
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

    /// Registers synchronization hooks between the default main and floating sessions.
    #[allow(non_snake_case)]
    pub fn setupCrossSessionSync(&mut self) {
        self.registerChatSelectionSync(ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING);
        self.registerTurnSync(ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING);
        self.registerTurnSync(ChatRuntimeSlot::FLOATING, ChatRuntimeSlot::MAIN);
    }

    /// Registers streaming-turn synchronization from one runtime slot to another.
    #[allow(non_snake_case)]
    pub fn registerTurnSync(&mut self, _sourceSlot: ChatRuntimeSlot, _targetSlot: ChatRuntimeSlot) {
    }

    /// Mirrors the selected main chat into the floating runtime.
    #[allow(non_snake_case)]
    pub fn syncMainChatSelectionToFloating(&mut self, chatId: String) {
        if chatId.trim().is_empty() {
            return;
        }
        self.syncChatSelection(ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING, chatId);
    }

    /// Registers chat-selection synchronization from one slot to another.
    #[allow(non_snake_case)]
    pub fn registerChatSelectionSync(
        &mut self,
        _sourceSlot: ChatRuntimeSlot,
        _targetSlot: ChatRuntimeSlot,
    ) {
    }

    /// Applies a chat selection change to the target runtime slot.
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
