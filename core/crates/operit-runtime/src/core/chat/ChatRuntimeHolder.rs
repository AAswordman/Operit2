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
    /// Returns whether proxy path segments identify a chat runtime owned by this holder.
    #[allow(non_snake_case)]
    pub fn matchesCorePath(pathSegments: &[String]) -> bool {
        chatRuntimeSlotFromPath(pathSegments).is_some()
    }

    /// Resolves proxy path segments to the chat service core owned by this holder.
    #[allow(non_snake_case)]
    pub fn coreForPath(&mut self, pathSegments: &[String]) -> Option<&mut ChatServiceCore> {
        let slot = chatRuntimeSlotFromPath(pathSegments)?;
        Some(self.getCore(slot))
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
        if targetCore.currentChatIdFlow().value().as_ref() == Some(&chatId) {
            return;
        }
        targetCore.switchChatLocal(chatId);
    }
}

/// Parses one holder-owned proxy path into its runtime slot.
#[allow(non_snake_case)]
fn chatRuntimeSlotFromPath(pathSegments: &[String]) -> Option<ChatRuntimeSlot> {
    match pathSegments {
        [root, slot] if root == "chatRuntimeHolder" => chatRuntimeSlot(slot, None),
        [root, holder, slot] if root == "application" && holder == "chatRuntimeHolder" => {
            chatRuntimeSlot(slot, None)
        }
        [root, slot, id] if root == "chatRuntimeHolder" => chatRuntimeSlot(slot, Some(id)),
        [root, holder, slot, id] if root == "application" && holder == "chatRuntimeHolder" => {
            chatRuntimeSlot(slot, Some(id))
        }
        _ => None,
    }
}

/// Parses one slot segment and optional instance identifier.
#[allow(non_snake_case)]
fn chatRuntimeSlot(slot: &str, id: Option<&String>) -> Option<ChatRuntimeSlot> {
    match (slot, id) {
        ("MAIN" | "main", None) => Some(ChatRuntimeSlot::MAIN),
        ("FLOATING" | "floating", None) => Some(ChatRuntimeSlot::FLOATING),
        ("DETACHED" | "detached", Some(id)) => Some(ChatRuntimeSlot::DETACHED(id.clone())),
        _ => None,
    }
}
