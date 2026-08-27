use std::collections::HashMap;

use crate::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::services::core::ChatHistoryDelegate::ChatSelectionMode;
use crate::services::ChatServiceCore::{ChatServiceCore, PendingChatQueueStore};
use operit_host_api::FileSystemHost;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_providers::runtime_support::ProviderRuntimeContext;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use std::sync::Arc;

#[derive(Clone)]
struct ChatRuntimeDependencies {
    toolHandler: AIToolHandler,
    providerRuntimeContext: ProviderRuntimeContext,
}

/// Builds chat service cores for each runtime slot.
pub struct ChatRuntimeCoreFactory {
    fileSystemHost: Arc<dyn FileSystemHost>,
    runtimeDependencies: Option<ChatRuntimeDependencies>,
    pendingQueueStore: Arc<PendingChatQueueStore>,
}

impl ChatRuntimeCoreFactory {
    /// Creates a factory used before host capabilities have been installed.
    pub fn bootstrap(fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self {
            fileSystemHost,
            runtimeDependencies: None,
            pendingQueueStore: Arc::new(PendingChatQueueStore::new()),
        }
    }

    /// Creates a factory that wires chat cores to runtime dependencies.
    pub fn new(
        fileSystemHost: Arc<dyn FileSystemHost>,
        toolHandler: AIToolHandler,
        providerRuntimeContext: ProviderRuntimeContext,
    ) -> Self {
        Self {
            fileSystemHost,
            runtimeDependencies: Some(ChatRuntimeDependencies {
                toolHandler,
                providerRuntimeContext,
            }),
            pendingQueueStore: Arc::new(PendingChatQueueStore::new()),
        }
    }

    /// Creates a chat service core configured for the requested slot.
    pub fn createCore(&self, slot: ChatRuntimeSlot) -> ChatServiceCore {
        let mut core = ChatServiceCore::newWithPendingQueueStore(
            match slot {
                ChatRuntimeSlot::MAIN => ChatSelectionMode::FOLLOW_GLOBAL,
                ChatRuntimeSlot::FLOATING | ChatRuntimeSlot::DETACHED(_) => {
                    ChatSelectionMode::LOCAL_ONLY
                }
            },
            self.fileSystemHost.clone(),
            self.pendingQueueStore.clone(),
        );
        if let Some(runtimeDependencies) = &self.runtimeDependencies {
            core.enhancedAiService = Some(EnhancedAIService::new(
                runtimeDependencies.toolHandler.clone(),
                runtimeDependencies.providerRuntimeContext.clone(),
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
    /// Resolves one generated proxy object id to the main chat service core.
    #[allow(non_snake_case)]
    pub fn coreForObjectId(&mut self, _objectId: u32) -> Option<&mut ChatServiceCore> {
        Some(self.getCore(ChatRuntimeSlot::MAIN))
    }

    /// Creates a holder using bootstrap cores without host-backed enhanced AI services.
    pub fn new(fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::bootstrap(fileSystemHost))
    }

    /// Creates a holder that injects runtime dependencies into newly created cores.
    #[allow(non_snake_case)]
    pub fn newWithRuntimeDependencies(
        fileSystemHost: Arc<dyn FileSystemHost>,
        toolHandler: AIToolHandler,
        providerRuntimeContext: ProviderRuntimeContext,
    ) -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::new(
            fileSystemHost,
            toolHandler,
            providerRuntimeContext,
        ))
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
}
