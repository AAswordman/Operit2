use std::collections::HashMap;

use crate::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::services::core::ChatHistoryDelegate::ChatSelectionMode;
use crate::services::core::MessageProcessingDelegate::ChatExecutionState;
use crate::services::ChatServiceCore::{ChatServiceCore, PendingChatQueueStore};
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_providers::runtime_support::ProviderRuntimeContext;
use operit_store::PreferencesDataStore::StateFlow;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_vfs::VisualFileSystem::VisualFileSystem;
use std::sync::{Arc, RwLock};

/// Shares each live core's authoritative execution flow without borrowing its runtime holder.
#[derive(Clone, Default)]
pub(crate) struct ChatRuntimeProcessingState {
    sources: Arc<RwLock<HashMap<ChatRuntimeSlot, StateFlow<HashMap<String, ChatExecutionState>>>>>,
}

impl ChatRuntimeProcessingState {
    /// Reads the same authoritative snapshot for occupancy and round reporting.
    #[allow(non_snake_case)]
    pub(crate) fn agentStatus(
        &self,
        chatId: &str,
    ) -> Result<operit_plugin_sdk::js_sdk::results::AgentStatusResultData, String> {
        let sources = self
            .sources
            .read()
            .map_err(|_| "Chat processing state registry lock poisoned".to_string())?;
        let state = sources
            .values()
            .filter_map(|source| source.value().get(chatId).cloned())
            .max_by_key(|state| (state.isProcessing(), state.currentRound));
        let isProcessing = state.as_ref().is_some_and(ChatExecutionState::isProcessing);
        Ok(operit_plugin_sdk::js_sdk::results::AgentStatusResultData {
            chatId: chatId.to_string(),
            state: if isProcessing { "processing" } else { "idle" }.to_string(),
            message: operit_plugin_sdk::js_sdk::JsOptional::Null,
            isIdle: !isProcessing,
            isProcessing,
            currentRound: state.as_ref().map_or(0, |state| state.currentRound),
            taskTitle: state.and_then(|state| state.taskTitle),
        })
    }
    /// Registers the actual core flow, including runtimes created after tools have been bound.
    fn register(
        &self,
        slot: ChatRuntimeSlot,
        source: StateFlow<HashMap<String, ChatExecutionState>>,
    ) {
        self.sources
            .write()
            .expect("Chat processing state registry lock poisoned")
            .insert(slot, source);
    }

    /// Removes stopped cores so retained worker clones cannot report obsolete runtime activity.
    fn clear(&self) {
        self.sources
            .write()
            .expect("Chat processing state registry lock poisoned")
            .clear();
    }
}

#[derive(Clone)]
struct ChatRuntimeDependencies {
    toolHandler: AIToolHandler,
    providerRuntimeContext: ProviderRuntimeContext,
}

/// Builds chat service cores for each runtime slot.
pub struct ChatRuntimeCoreFactory {
    filesystem: Arc<VisualFileSystem>,
    runtimeDependencies: Option<ChatRuntimeDependencies>,
    pendingQueueStore: Arc<PendingChatQueueStore>,
}

impl ChatRuntimeCoreFactory {
    /// Creates a factory used before host capabilities have been installed.
    pub fn bootstrap(filesystem: Arc<VisualFileSystem>) -> Self {
        Self {
            filesystem,
            runtimeDependencies: None,
            pendingQueueStore: Arc::new(PendingChatQueueStore::new()),
        }
    }

    /// Creates a factory that wires chat cores to runtime dependencies.
    pub fn new(
        filesystem: Arc<VisualFileSystem>,
        toolHandler: AIToolHandler,
        providerRuntimeContext: ProviderRuntimeContext,
    ) -> Self {
        Self {
            filesystem,
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
            match slot {
                ChatRuntimeSlot::MAIN => Some(operit_plugin_sdk::js_sdk::chat::ChatRuntime::Main),
                ChatRuntimeSlot::FLOATING => {
                    Some(operit_plugin_sdk::js_sdk::chat::ChatRuntime::Floating)
                }
                ChatRuntimeSlot::DETACHED(_) => None,
            },
            self.filesystem.clone(),
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
    processingState: ChatRuntimeProcessingState,
}

impl ChatRuntimeHolder {
    /// Commits a prepared reply once and publishes its canonical record to every open runtime slot.
    #[allow(non_snake_case)]
    pub(crate) fn commitPreparedAssistantMessage(
        &mut self,
        chatId: &str,
        message: operit_model::ChatMessage::ChatMessage,
    ) -> Result<bool, String> {
        let history = &mut self.getCore(ChatRuntimeSlot::MAIN).chatHistoryDelegate;
        let (duplicate, message) = history.commitPreparedAssistantMessageSnapshot(chatId, message)?;
        for core in self.cores.values() {
            core.chatHistoryDelegate.publishChatMessage(chatId, message.clone());
        }
        Ok(duplicate)
    }

    /// Resolves one generated proxy object id to the main chat service core.
    #[allow(non_snake_case)]
    pub fn coreForTarget(&mut self, _target: &str) -> Option<&mut ChatServiceCore> {
        Some(self.getCore(ChatRuntimeSlot::MAIN))
    }

    /// Resolves a detached window's stable slot identity to its local chat core.
    ///
    /// The generated proxy uses the slot id as an instance argument because all
    /// detached windows share the same generated object schema.
    #[allow(non_snake_case)]
    pub fn coreForInstanceId(&mut self, instanceId: String) -> Option<&mut ChatServiceCore> {
        Some(self.getCore(ChatRuntimeSlot::DETACHED(instanceId)))
    }

    /// Creates a holder using bootstrap cores without host-backed enhanced AI services.
    pub fn new(filesystem: Arc<VisualFileSystem>) -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::bootstrap(filesystem))
    }

    /// Creates a holder that injects runtime dependencies into newly created cores.
    #[allow(non_snake_case)]
    pub fn newWithRuntimeDependencies(
        filesystem: Arc<VisualFileSystem>,
        toolHandler: AIToolHandler,
        providerRuntimeContext: ProviderRuntimeContext,
    ) -> Self {
        Self::newWithFactory(ChatRuntimeCoreFactory::new(
            filesystem,
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
            processingState: ChatRuntimeProcessingState::default(),
        };
        for slot in [ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING] {
            holder.getCore(slot);
        }
        holder.observeStats();
        holder
    }

    /// Returns a live read handle whose membership follows this holder's core lifecycle.
    #[allow(non_snake_case)]
    pub(crate) fn processingState(&self) -> ChatRuntimeProcessingState {
        self.processingState.clone()
    }

    /// Stops runtime cores and removes their execution flows from independent status readers.
    #[allow(non_snake_case)]
    pub(crate) fn clearCores(&mut self) {
        self.processingState.clear();
        self.cores.clear();
        self.activeConversationCount = 0;
        self.currentSessionToolCount = 0;
    }

    /// Borrows the actual main delegate's canonical manager for initialization-time record and execution-lease sharing.
    #[allow(non_snake_case)]
    pub(crate) fn chatHistoryManager(
        &self,
    ) -> Result<&operit_store::repository::ChatHistoryManager::ChatHistoryManager, String> {
        self.cores
            .get(&ChatRuntimeSlot::MAIN)
            .map(|core| &core.chatHistoryDelegate.chatHistoryManager)
            .ok_or_else(|| "Chat runtime has no initialized main record manager".to_string())
    }

    /// Returns the core for a slot, creating it from the factory when first used.
    #[allow(non_snake_case)]
    pub fn getCore(&mut self, slot: ChatRuntimeSlot) -> &mut ChatServiceCore {
        if !self.cores.contains_key(&slot) {
            let core = self.coreFactory.createCore(slot.clone());
            self.processingState.register(
                slot.clone(),
                core.messageProcessingDelegate.executionStateByChatIdFlow(),
            );
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
