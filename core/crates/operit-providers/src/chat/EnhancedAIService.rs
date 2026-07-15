use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use operit_store::PreferencesDataStore::mutableStateFlow;
use operit_store::PreferencesDataStore::MutableStateFlow;
use regex::Regex;
use serde_json::{json, Value};

use crate::chat::config::SystemPromptConfig::{
    PackageInfo, SystemPromptConfig, SystemPromptOptions, SystemPromptWithCustomOptions,
    ToolExposureMode as SystemToolExposureMode,
};
use crate::chat::config::SystemToolPrompts::SystemToolPrompts;
use crate::chat::enhance::ConversationService::{
    ConversationService, HistoryHookContext, PrepareConversationHistoryRequest,
    PromptHistoryHookDispatcher, SystemPromptComposer, ToolExposureMode,
};
use crate::chat::enhance::MultiServiceManager::{MultiServiceManager, SharedAIServiceHandle};
use crate::chat::hooks::PromptHookRegistry::{PromptHookContext, PromptHookRegistry};
use crate::chat::library::MemoryLibrary::{promptTurnsToMemoryPairs, MemoryLibrary};
use crate::chat::llmprovider::AIService::{
    response_stream_from_chunks, AiServiceError, SendMessageRequest, SharedAiResponseStream,
    TokenCounts,
};
use crate::runtime_support::{ProviderRuntimeContext, ProviderRuntimeSupport};
use operit_model::CharacterCard::CharacterCardMemoryBindingMode;
use operit_model::FunctionType::FunctionType;
use operit_model::InputProcessingState::InputProcessingState;
use operit_model::ModelConfigData::ResolvedModelConfig;
use operit_model::ModelParameter::ModelParameter;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::{ToolParameterSchema, ToolPrompt};
use operit_store::repository::UsageStatisticsStore::{UsageRequestSource, UsageStatisticsStore};
use operit_store::repository::UserMarkdownRepository::UserMarkdownRepository;
use operit_tools::tools::climode::CliToolModeSupport::{
    CliToolModeSupport, ToolExposureMode as ResolvedToolExposureMode,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::ConversationMarkupManager::{
    ConversationMarkupManager, ENHANCED_PURE_THINKING_ONLY_WARNING,
    ENHANCED_TRUNCATED_TOOL_CALL_WARNING,
};
use operit_tools::ToolExecutionManager::{
    AITool as RuntimeAITool, ToolExecutionManager, ToolExposureMode as RuntimeToolExposureMode,
};
use operit_util::stream::RevisableTextStream::RevisableTextStreamLike;
use operit_util::stream::RevisableTextStream::{with_event_channel_shared, TextStreamEventCarrier};
use operit_util::stream::Stream::{FnStream, Stream};
use operit_util::AppLogger::AppLogger;
use operit_util::ChatMarkupRegex::{attr_value, ChatMarkupRegex};
use operit_util::ChatUtils::ChatUtils;
use operit_util::OperitPaths::{characterMemoryOwnerKey, sharedMemoryOwnerKey};

const TAG: &str = "EnhancedAIService";

pub struct EnhancedAIService {
    pub multi_service_manager: MultiServiceManager,
    pub init_scope: InitScopeMirror,
    pub init_mutex: InitMutexMirror,
    pub conversation_service: ConversationService,
    pub file_binding_service: FileBindingServiceMirror,
    pub tool_handler: AIToolHandler,
    pub input_processing_state: MutableStateFlow<InputProcessingState>,
    pub request_window_estimate_flow: MutableStateFlow<Option<i32>>,
    pub api_preferences: ApiPreferencesMirror,
    pub character_card_tool_access_resolver: CharacterCardToolAccessResolverMirror,
    pub tool_processing_scope: ToolProcessingScopeMirror,
    pub package_manager: PackageManagerMirror,
    pub provider_runtime_context: ProviderRuntimeContext,
    pub shared_state: Arc<Mutex<EnhancedAISharedState>>,
}

#[derive(Clone, Debug)]
pub struct EnhancedAISharedState {
    pub is_service_manager_initialized: bool,
    pub per_request_token_counts: Option<(i32, i32)>,
    pub request_window_estimate: Option<i32>,
    pub active_execution_contexts: BTreeMap<i32, MessageExecutionContext>,
    pub next_execution_context_id: i32,
    pub tool_execution_jobs: BTreeMap<String, ToolExecutionJobMirror>,
    pub accumulated_input_token_count: i32,
    pub accumulated_output_token_count: i32,
    pub accumulated_cached_input_token_count: i32,
    pub current_request_input_token_count: i32,
    pub current_request_output_token_count: i32,
    pub current_request_cached_input_token_count: i32,
    pub current_response_callback_registered: bool,
    pub current_complete_callback_registered: bool,
    pub last_reply_content: Option<String>,
    pub last_provider_model: Option<String>,
    pub last_turn_token_snapshot: Option<TurnTokenSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnTokenSnapshot {
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub cachedInputTokens: i32,
}

pub trait SendMessageCallbacks: Send + Sync {
    fn onNonFatalError(&self, _error: String) {}

    fn onTokenLimitExceeded(&self) {}

    fn onToolInvocation(&self, _toolName: String) {}

    fn onInputProcessingStateChanged(&self, _state: InputProcessingState) {}
}

pub struct SendMessageOptions {
    pub message: String,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
    pub chatId: Option<String>,
    pub chatHistory: Vec<PromptTurn>,
    pub workspacePath: Option<String>,
    pub functionType: FunctionType,
    pub promptFunctionType: PromptFunctionType,
    pub enableThinking: bool,
    pub enableMemoryAutoUpdate: bool,
    pub onNonFatalError: Option<fn(String)>,
    pub onTokenLimitExceeded: Option<fn()>,
    pub customSystemPromptTemplate: Option<String>,
    pub isSubTask: bool,
    pub characterName: Option<String>,
    pub avatarUri: Option<String>,
    pub roleCardId: Option<String>,
    pub enableGroupOrchestrationHint: bool,
    pub groupParticipantNamesText: Option<String>,
    pub proxySenderName: Option<String>,
    pub callbacks: Option<Arc<dyn SendMessageCallbacks + Send + Sync>>,
    pub onToolInvocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
    pub notifyReplyOverride: Option<bool>,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub stream: bool,
    pub disableWarning: bool,
}

impl SendMessageOptions {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            maxTokens: 0,
            tokenUsageThreshold: 0.0,
            chatId: None,
            chatHistory: Vec::new(),
            workspacePath: None,
            functionType: FunctionType::CHAT,
            promptFunctionType: PromptFunctionType::CHAT,
            enableThinking: false,
            enableMemoryAutoUpdate: true,
            onNonFatalError: None,
            onTokenLimitExceeded: None,
            customSystemPromptTemplate: None,
            isSubTask: false,
            characterName: None,
            avatarUri: None,
            roleCardId: None,
            enableGroupOrchestrationHint: false,
            groupParticipantNamesText: None,
            proxySenderName: None,
            callbacks: None,
            onToolInvocation: None,
            notifyReplyOverride: None,
            chatProviderIdOverride: None,
            chatModelIdOverride: None,
            stream: true,
            disableWarning: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MessageExecutionContext {
    pub executionId: i32,
    pub streamBuffer: String,
    pub roundManager: ConversationRoundManagerMirror,
    pub isConversationActive: bool,
    pub conversationHistory: Vec<PromptTurn>,
    pub workspacePath: Option<String>,
    pub eventChannel: MutableSharedStreamMirror<TextStreamEventMirror>,
}

impl MessageExecutionContext {
    pub fn new(
        executionId: i32,
        conversationHistory: Vec<PromptTurn>,
        workspacePath: Option<String>,
        eventChannel: MutableSharedStreamMirror<TextStreamEventMirror>,
    ) -> Self {
        Self {
            executionId,
            streamBuffer: String::new(),
            roundManager: ConversationRoundManagerMirror::new(),
            isConversationActive: true,
            conversationHistory,
            workspacePath,
            eventChannel,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendMessageLifecycleStage {
    EnsureInitialized,
    StartAiService,
    SetProcessingState,
    PrepareConversationHistory,
    SyncPreparedHistoryToExecutionContext,
    SetConnectingState,
    GetModelParametersForFunction,
    GetAIServiceForFunction,
    ClearPerRequestTokenCounts,
    GetAvailableToolsForFunction,
    BeforeFinalizePromptHook,
    BeforeSendToModelHook,
    StripGeminiThoughtSignatureMeta,
    ApplyFinalizedCurrentUserTurn,
    SyncRequestHistoryToExecutionContext,
    EstimatePreparedRequestWindow,
    SendMessageRequest,
    StartAssistantResponseRound,
    CollectResponseStream,
    ExtractToolInvocations,
    ExecuteToolInvocations,
    ProcessToolResults,
    PersistTokenUsage,
    ProcessStreamCompletion,
    UnregisterExecutionContext,
    StopAiService,
}

#[derive(Clone, Debug)]
pub struct SendMessageExecution {
    pub processedInput: String,
    pub requestHistory: Vec<PromptTurn>,
    pub responseChunks: Vec<String>,
    pub tokenSnapshot: TurnTokenSnapshot,
    pub requestWindowSize: i32,
    pub providerModel: String,
    pub lifecycle: Vec<SendMessageLifecycleStage>,
}

pub struct SendMessageRuntime {
    pub activePromptMetadata: BTreeMap<String, String>,
    pub useEnglish: bool,
    pub userPreferencesText: String,
    pub introPrompt: String,
    pub waifuRulesText: String,
    pub avatarMoodRulesText: String,
    pub disableUserPreferenceDescription: bool,
    pub aiName: String,
    pub hasImageRecognition: bool,
    pub hasAudioRecognition: bool,
    pub hasVideoRecognition: bool,
    pub chatModelHasDirectAudio: bool,
    pub chatModelHasDirectVideo: bool,
    pub chatModelHasDirectImage: bool,
    pub useToolCallApi: bool,
    pub toolExposureMode: ToolExposureMode,
    pub modelConfig: ResolvedModelConfig,
    pub modelParameters: Vec<ModelParameter<Value>>,
    pub availableTools: Vec<ToolPrompt>,
    pub aiService: SharedAIServiceHandle,
}

#[derive(Clone, Debug)]
pub struct InitScopeMirror;

#[derive(Clone, Debug)]
pub struct InitMutexMirror;

#[derive(Clone, Debug)]
pub struct FileBindingServiceMirror;

#[derive(Clone, Debug)]
pub struct ApiPreferencesMirror;

#[derive(Clone, Debug)]
pub struct CharacterCardToolAccessResolverMirror;

#[derive(Clone, Debug)]
pub struct PackageManagerMirror;

#[derive(Clone, Debug)]
pub struct ToolProcessingScopeMirror;

#[derive(Clone, Debug)]
pub struct ToolExecutionJobMirror;

#[derive(Clone, Debug)]
pub struct MutableSharedStreamMirror<T> {
    pub replay: usize,
    pub events: Vec<T>,
}

impl<T> MutableSharedStreamMirror<T> {
    pub fn new(replay: usize) -> Self {
        Self {
            replay,
            events: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextStreamEventMirror;

#[derive(Clone, Debug)]
pub struct ConversationRoundManagerMirror {
    pub content: String,
    pub roundIndex: i32,
}

impl ConversationRoundManagerMirror {
    /// Creates an empty mirror for conversation round content.
    pub fn new() -> Self {
        Self {
            content: String::new(),
            roundIndex: 0,
        }
    }

    /// Starts a new mirrored conversation round and clears mirrored content.
    pub fn startNewRound(&mut self) {
        self.roundIndex += 1;
        self.content.clear();
    }

    /// Replaces the mirrored content for the current conversation round.
    pub fn updateContent(&mut self, content: String) {
        self.content = content;
    }

    /// Appends text to the mirrored content for the current conversation round.
    pub fn appendContent(&mut self, content: &str) {
        self.content.push_str(content);
    }

    /// Returns the mirrored content formatted for display.
    pub fn getDisplayContent(&self) -> String {
        self.content.clone()
    }

    /// Returns the mirrored content for the current conversation round.
    pub fn getCurrentRoundContent(&self) -> String {
        self.content.clone()
    }
}

pub struct RuntimePromptHistoryHooks;

impl PromptHistoryHookDispatcher for RuntimePromptHistoryHooks {
    fn dispatch_prompt_history_hooks(&self, context: HistoryHookContext) -> HistoryHookContext {
        let dispatched = PromptHookRegistry::dispatchPromptHistoryHooks(PromptHookContext {
            stage: context.stage.clone(),
            chat_id: context.chat_id.clone(),
            function_type: None,
            prompt_function_type: Some(context.prompt_function_type.clone()),
            use_english: context.use_english,
            raw_input: None,
            processed_input: Some(context.processed_input.clone()),
            chat_history: context.chat_history.clone(),
            prepared_history: context.prepared_history.clone(),
            system_prompt: None,
            tool_prompt: None,
            model_parameters: Vec::new(),
            available_tools: Vec::new(),
            metadata: btree_to_value_map(&context.metadata),
        });

        HistoryHookContext {
            stage: dispatched.stage,
            chat_id: dispatched.chat_id,
            prompt_function_type: dispatched
                .prompt_function_type
                .expect("PromptHistoryHook must preserve prompt_function_type"),
            processed_input: dispatched
                .processed_input
                .expect("PromptHistoryHook must preserve processed_input"),
            chat_history: dispatched.chat_history,
            prepared_history: dispatched.prepared_history,
            use_english: dispatched.use_english,
            metadata: value_to_btree_map(dispatched.metadata),
        }
    }
}

pub struct RuntimeSystemPromptComposer {
    tool_handler: AIToolHandler,
    provider_runtime_context: ProviderRuntimeContext,
}

impl SystemPromptComposer for RuntimeSystemPromptComposer {
    fn get_system_prompt_with_custom_prompts(
        &self,
        request: &PrepareConversationHistoryRequest,
        use_english: bool,
    ) -> String {
        let custom_system_prompt_template = match &request.custom_system_prompt_template {
            Some(value) => value.clone(),
            None => String::new(),
        };
        let group_participant_names_text = match &request.group_participant_names_text {
            Some(value) => value.clone(),
            None => String::new(),
        };
        let host_environment = self.tool_handler.getHostEnvironmentDescriptor();
        let package_manager = self.tool_handler.getOrCreatePackageManager();
        let package_manager_guard = package_manager
            .lock()
            .expect("package manager mutex poisoned");
        let enabled_packages = package_manager_guard
            .getEnabledPackageNames()
            .into_iter()
            .filter_map(|package_name| {
                package_manager_guard
                    .getEffectivePackageTools(&package_name)
                    .filter(|_| !package_manager_guard.isToolPkgContainer(&package_name))
                    .map(|tool_package| PackageInfo {
                        name: package_name,
                        description: tool_package.description.resolve(use_english),
                    })
            })
            .collect::<Vec<_>>();
        let mcp_servers = package_manager_guard
            .getAvailableServerPackages()
            .into_iter()
            .map(|(name, server_config)| PackageInfo {
                name,
                description: server_config.description,
            })
            .collect::<Vec<_>>();
        drop(package_manager_guard);
        let skill_packages = self
            .provider_runtime_context
            .support()
            .aiVisibleSkillPackages()
            .expect("provider runtime support must provide skill packages")
            .into_iter()
            .map(|package| PackageInfo {
                name: package.name,
                description: package.description,
            })
            .collect::<Vec<_>>();
        SystemPromptConfig::getSystemPromptWithCustomPrompts(SystemPromptWithCustomOptions {
            base: SystemPromptOptions {
                chat_id: request.chat_id.clone(),
                workspace_path: request.workspace_path.clone(),
                use_english,
                custom_system_prompt_template,
                enable_tools: true,
                has_image_recognition: request.has_image_recognition,
                chat_model_has_direct_image: request.chat_model_has_direct_image,
                has_audio_recognition: request.has_audio_recognition,
                has_video_recognition: request.has_video_recognition,
                chat_model_has_direct_audio: request.chat_model_has_direct_audio,
                chat_model_has_direct_video: request.chat_model_has_direct_video,
                use_tool_call_api: request.use_tool_call_api,
                tool_exposure_mode: match request.tool_exposure_mode {
                    ToolExposureMode::Full => SystemToolExposureMode::FULL,
                    ToolExposureMode::Cli => SystemToolExposureMode::CLI,
                },
                host_environment,
                enabled_packages,
                mcp_servers,
                skill_packages,
                hook_metadata: btree_to_value_map(&request.active_prompt_metadata),
                ..SystemPromptOptions::default()
            },
            custom_intro_prompt: request.intro_prompt.clone(),
            enable_group_orchestration_hint: request.enable_group_orchestration_hint,
            group_orchestration_role_name: request.ai_name.clone(),
            group_participant_names_text,
        })
    }
}

impl EnhancedAIService {
    /// Creates the enhanced AI service with explicit runtime dependencies.
    pub fn new(
        tool_handler: AIToolHandler,
        provider_runtime_context: ProviderRuntimeContext,
    ) -> Self {
        let conversation_service = ConversationService::new(provider_runtime_context.clone());
        Self {
            multi_service_manager: MultiServiceManager::from_runtime_context(
                provider_runtime_context.clone(),
            )
            .expect("provider runtime support must provide a data directory"),
            init_scope: InitScopeMirror,
            init_mutex: InitMutexMirror,
            conversation_service,
            file_binding_service: FileBindingServiceMirror,
            tool_handler,
            input_processing_state: mutableStateFlow(InputProcessingState::Idle),
            request_window_estimate_flow: mutableStateFlow(None),
            api_preferences: ApiPreferencesMirror,
            character_card_tool_access_resolver: CharacterCardToolAccessResolverMirror,
            tool_processing_scope: ToolProcessingScopeMirror,
            package_manager: PackageManagerMirror,
            provider_runtime_context,
            shared_state: Arc::new(Mutex::new(EnhancedAISharedState {
                is_service_manager_initialized: false,
                per_request_token_counts: None,
                request_window_estimate: None,
                active_execution_contexts: BTreeMap::new(),
                next_execution_context_id: 0,
                tool_execution_jobs: BTreeMap::new(),
                accumulated_input_token_count: 0,
                accumulated_output_token_count: 0,
                accumulated_cached_input_token_count: 0,
                current_request_input_token_count: 0,
                current_request_output_token_count: 0,
                current_request_cached_input_token_count: 0,
                current_response_callback_registered: false,
                current_complete_callback_registered: false,
                last_reply_content: None,
                last_provider_model: None,
                last_turn_token_snapshot: None,
            })),
        }
    }

    fn shared_state(&self) -> std::sync::MutexGuard<'_, EnhancedAISharedState> {
        self.shared_state
            .lock()
            .expect("EnhancedAIService shared_state mutex poisoned")
    }

    /// Initializes the service manager once before provider access.
    pub fn ensureInitialized(&mut self) {
        if self.shared_state().is_service_manager_initialized {
            return;
        }
        self.multi_service_manager
            .initialize()
            .expect("MultiServiceManager initialization must succeed");
        self.shared_state().is_service_manager_initialized = true;
    }

    /// Returns the model service configured for a functional prompt role.
    pub fn getAIServiceForFunction(
        &mut self,
        _functionType: FunctionType,
        _chatProviderIdOverride: Option<String>,
        _chatModelIdOverride: Option<String>,
        runtime: &SendMessageRuntime,
    ) -> SharedAIServiceHandle {
        self.ensureInitialized();
        runtime.aiService.clone()
    }

    pub fn getProviderAndModelForFunction(&self, providerModel: &str) -> (String, String) {
        let colonIndex = providerModel
            .find(':')
            .expect("providerModel must contain ':'");
        (
            providerModel[..colonIndex].to_string(),
            providerModel[colonIndex + 1..].to_string(),
        )
    }

    pub fn getModelConfigForFunction(
        &mut self,
        _functionType: FunctionType,
        _chatProviderIdOverride: Option<String>,
        _chatModelIdOverride: Option<String>,
        runtime: &SendMessageRuntime,
    ) -> ResolvedModelConfig {
        self.ensureInitialized();
        runtime.modelConfig.clone()
    }

    pub async fn refreshServiceForFunction(&mut self, functionType: FunctionType) {
        self.ensureInitialized();
        self.multi_service_manager
            .refreshServiceForFunction(functionType)
            .await
            .expect("refreshServiceForFunction must succeed");
    }

    pub async fn refreshAllServices(&mut self) {
        self.ensureInitialized();
        self.multi_service_manager
            .refreshAllServices()
            .await
            .expect("refreshAllServices must succeed");
    }

    pub fn getModelParametersForFunction(
        &mut self,
        _functionType: FunctionType,
        _chatProviderIdOverride: Option<String>,
        _chatModelIdOverride: Option<String>,
        runtime: &SendMessageRuntime,
    ) -> Vec<ModelParameter<Value>> {
        self.ensureInitialized();
        runtime.modelParameters.clone()
    }

    pub fn publishRequestWindowEstimate(&mut self, windowSize: i32) {
        self.shared_state().request_window_estimate = Some(windowSize);
        self.request_window_estimate_flow
            .set_value(Some(windowSize));
    }

    pub async fn estimatePreparedRequestWindow(
        &mut self,
        serviceForFunction: SharedAIServiceHandle,
        preparedHistory: &[PromptTurn],
        availableTools: &[ToolPrompt],
        publishEstimate: bool,
    ) -> Result<i32, AiServiceError> {
        let windowSize = {
            let service = serviceForFunction.lock().await;
            service
                .calculate_input_tokens(preparedHistory, availableTools)
                .await?
        };
        if publishEstimate {
            self.publishRequestWindowEstimate(windowSize);
        }
        Ok(windowSize)
    }

    pub fn applyPromptFinalizeHooks(
        &self,
        initialContext: PromptHookContext,
        dispatchHooks: fn(PromptHookContext) -> PromptHookContext,
    ) -> PromptHookContext {
        dispatchHooks(initialContext)
    }

    pub fn bypassPromptHooks(&self, context: PromptHookContext) -> PromptHookContext {
        context
    }

    pub fn buildPromptFinalizeMetadata(
        &self,
        chatId: Option<String>,
        roleCardId: Option<String>,
        workspacePath: Option<String>,
        enableThinking: bool,
        stream: bool,
        isSubTask: bool,
    ) -> HashMap<String, Value> {
        HashMap::from([
            ("workspacePath".to_string(), json!(workspacePath)),
            ("enableThinking".to_string(), json!(enableThinking)),
            ("stream".to_string(), json!(stream)),
            ("isSubTask".to_string(), json!(isSubTask)),
            ("chatId".to_string(), json!(chatId)),
            ("roleCardId".to_string(), json!(roleCardId)),
        ])
    }

    pub fn applyFinalizedCurrentUserTurn(
        &self,
        preparedHistory: Vec<PromptTurn>,
        originalCurrentMessage: &str,
        finalizedCurrentMessage: &str,
    ) -> Vec<PromptTurn> {
        apply_finalized_current_user_turn(
            preparedHistory,
            originalCurrentMessage,
            finalizedCurrentMessage,
        )
    }

    pub fn prepareConversationHistory(
        &mut self,
        chatHistory: Vec<PromptTurn>,
        processedInput: String,
        chatId: Option<String>,
        workspacePath: Option<String>,
        promptFunctionType: PromptFunctionType,
        customSystemPromptTemplate: Option<String>,
        roleCardId: Option<String>,
        enableGroupOrchestrationHint: bool,
        groupParticipantNamesText: Option<String>,
        proxySenderName: Option<String>,
        isSubTask: bool,
        functionType: FunctionType,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        runtime: &SendMessageRuntime,
    ) -> Vec<PromptTurn> {
        let config = self.getModelConfigForFunction(
            functionType,
            chatProviderIdOverride,
            chatModelIdOverride,
            runtime,
        );
        let useToolCallApi = config.capabilities.toolCall;
        let chatModelHasDirectImage = config.capabilities.directImage;
        let chatModelHasDirectAudio = config.capabilities.directAudio;
        let chatModelHasDirectVideo = config.capabilities.directVideo;

        let history_hooks = RuntimePromptHistoryHooks;
        let system_prompt_composer = RuntimeSystemPromptComposer {
            tool_handler: self.tool_handler.clone(),
            provider_runtime_context: self.provider_runtime_context.clone(),
        };
        self.conversation_service.prepare_conversation_history(
            PrepareConversationHistoryRequest {
                chat_history: chatHistory,
                processed_input: processedInput,
                chat_id: chatId,
                workspace_path: workspacePath,
                prompt_function_type: prompt_function_type_name(&promptFunctionType).to_string(),
                custom_system_prompt_template: customSystemPromptTemplate,
                role_card_id: roleCardId,
                enable_group_orchestration_hint: enableGroupOrchestrationHint,
                group_participant_names_text: groupParticipantNamesText,
                proxy_sender_name: proxySenderName,
                has_image_recognition: !isSubTask && runtime.hasImageRecognition,
                has_audio_recognition: !isSubTask && runtime.hasAudioRecognition,
                has_video_recognition: !isSubTask && runtime.hasVideoRecognition,
                chat_model_has_direct_audio: chatModelHasDirectAudio,
                chat_model_has_direct_video: chatModelHasDirectVideo,
                use_tool_call_api: useToolCallApi,
                chat_model_has_direct_image: chatModelHasDirectImage,
                tool_exposure_mode: runtime.toolExposureMode.clone(),
                active_prompt_metadata: runtime.activePromptMetadata.clone(),
                user_preferences_text: runtime.userPreferencesText.clone(),
                intro_prompt: runtime.introPrompt.clone(),
                waifu_rules_text: runtime.waifuRulesText.clone(),
                avatar_mood_rules_text: runtime.avatarMoodRulesText.clone(),
                disable_user_preference_description: runtime.disableUserPreferenceDescription,
                ai_name: runtime.aiName.clone(),
            },
            &history_hooks,
            &system_prompt_composer,
            runtime.useEnglish,
        )
    }

    pub async fn generateSummary(
        &mut self,
        messages: Vec<(String, String)>,
        previousSummary: Option<String>,
    ) -> Result<String, AiServiceError> {
        let mut multiServiceManager = self.multi_service_manager.clone();
        multiServiceManager.initialize()?;
        self.conversation_service
            .generateSummary(messages, previousSummary, &mut multiServiceManager)
            .await
    }

    pub async fn generateSummaryFromPromptTurns(
        &mut self,
        messages: Vec<PromptTurn>,
        previousSummary: Option<String>,
    ) -> Result<String, AiServiceError> {
        let mut multiServiceManager = self.multi_service_manager.clone();
        multiServiceManager.initialize()?;
        self.conversation_service
            .generateSummaryFromPromptTurns(messages, previousSummary, &mut multiServiceManager)
            .await
    }

    pub fn getAvailableToolsForFunction(
        &mut self,
        functionType: FunctionType,
        _chatId: Option<String>,
        _promptFunctionType: Option<PromptFunctionType>,
        _roleCardId: Option<String>,
        _chatProviderIdOverride: Option<String>,
        _chatModelIdOverride: Option<String>,
        runtime: &SendMessageRuntime,
    ) -> Vec<ToolPrompt> {
        if !runtime.availableTools.is_empty() {
            return runtime.availableTools.clone();
        }
        if functionType != FunctionType::CHAT || !runtime.modelConfig.capabilities.toolCall {
            return Vec::new();
        }
        self.tool_handler.registerDefaultTools();
        if runtime.toolExposureMode == ToolExposureMode::Cli {
            return CliToolModeSupport::buildCliPublicToolPrompts(runtime.useEnglish);
        }
        let host_environment = self.tool_handler.getHostEnvironmentDescriptor();
        let registered_tool_names = self.tool_handler.getAllToolNames();
        let categories = if runtime.useEnglish {
            SystemToolPrompts::getAIAllCategoriesEnForHost(
                false,
                runtime.chatModelHasDirectImage,
                false,
                false,
                runtime.chatModelHasDirectAudio,
                runtime.chatModelHasDirectVideo,
                &[],
                &host_environment,
            )
        } else {
            SystemToolPrompts::getAIAllCategoriesCnForHost(
                false,
                runtime.chatModelHasDirectImage,
                false,
                false,
                runtime.chatModelHasDirectAudio,
                runtime.chatModelHasDirectVideo,
                &[],
                &host_environment,
            )
        };
        let mut available_tools = categories
            .into_iter()
            .flat_map(|category| category.tools)
            .filter(|tool| registered_tool_names.contains(&tool.name))
            .map(systemToolPromptToModelToolPrompt)
            .collect::<Vec<_>>();
        available_tools.push(buildPackageProxyToolPrompt());
        available_tools
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn estimateRequestWindowFromMemory(
        &mut self,
        message: String,
        chatHistory: Vec<PromptTurn>,
        chatId: Option<String>,
        workspacePath: Option<String>,
        promptFunctionType: PromptFunctionType,
        roleCardId: Option<String>,
        enableGroupOrchestrationHint: bool,
        groupParticipantNamesText: Option<String>,
        proxySenderName: Option<String>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        publishEstimate: bool,
        mut runtime: SendMessageRuntime,
    ) -> Result<i32, AiServiceError> {
        self.ensureInitialized();
        let preparedHistory = self.prepareConversationHistory(
            chatHistory,
            message.clone(),
            chatId.clone(),
            workspacePath,
            promptFunctionType.clone(),
            None,
            roleCardId.clone(),
            enableGroupOrchestrationHint,
            groupParticipantNamesText,
            proxySenderName,
            false,
            FunctionType::CHAT,
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            &runtime,
        );
        let availableTools = self.getAvailableToolsForFunction(
            FunctionType::CHAT,
            chatId.clone(),
            Some(promptFunctionType.clone()),
            roleCardId,
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            &runtime,
        );
        let serviceForFunction = self.getAIServiceForFunction(
            FunctionType::CHAT,
            chatProviderIdOverride,
            chatModelIdOverride,
            &mut runtime,
        );
        self.estimatePreparedRequestWindow(
            serviceForFunction,
            &preparedHistory,
            &availableTools,
            publishEstimate,
        )
        .await
    }

    pub fn registerExecutionContext(&mut self, context: MessageExecutionContext) {
        self.shared_state()
            .active_execution_contexts
            .insert(context.executionId, context);
    }

    pub fn unregisterExecutionContext(&mut self, context: &MessageExecutionContext) {
        self.shared_state()
            .active_execution_contexts
            .remove(&context.executionId);
    }

    pub fn invalidateExecutionContext(
        &mut self,
        context: &mut MessageExecutionContext,
        reason: String,
    ) {
        AppLogger::d(
            TAG,
            &format!(
                "执行上下文已失效: id={}, reason={}",
                context.executionId, reason
            ),
        );
        context.isConversationActive = false;
        if let Some(active) = self
            .shared_state()
            .active_execution_contexts
            .get_mut(&context.executionId)
        {
            active.isConversationActive = false;
        }
    }

    pub fn invalidateAllExecutionContexts(&mut self, reason: String) {
        AppLogger::d(TAG, &format!("准备失效全部执行上下文: reason={}", reason));
        let ids = self
            .shared_state()
            .active_execution_contexts
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for id in ids {
            if let Some(active) = self.shared_state().active_execution_contexts.get_mut(&id) {
                active.isConversationActive = false;
            }
        }
        let _ = reason;
    }

    fn isExecutionContextActiveInSharedState(
        shared_state: &Arc<Mutex<EnhancedAISharedState>>,
        context: &MessageExecutionContext,
    ) -> bool {
        context.isConversationActive
            && shared_state
                .lock()
                .expect("EnhancedAIService shared_state mutex poisoned")
                .active_execution_contexts
                .get(&context.executionId)
                .map(|active| active.isConversationActive)
                .expect("execution context must be registered")
    }

    pub fn isExecutionContextActive(&self, context: &MessageExecutionContext) -> bool {
        Self::isExecutionContextActiveInSharedState(&self.shared_state, context)
    }

    pub fn startAssistantResponseRound(&mut self, context: &mut MessageExecutionContext) {
        context.roundManager.startNewRound();
        context.streamBuffer.clear();
    }

    pub fn setInputProcessingState(&mut self, newState: InputProcessingState) {
        self.input_processing_state.set_value(newState);
    }

    pub fn inputProcessingState(&self) -> MutableStateFlow<InputProcessingState> {
        self.input_processing_state.clone()
    }

    #[allow(non_snake_case)]
    pub fn requestWindowEstimateFlow(&self) -> MutableStateFlow<Option<i32>> {
        self.request_window_estimate_flow.clone()
    }

    pub fn startAiService(&mut self, _characterName: Option<String>, _avatarUri: Option<String>) {}

    pub fn stopAiService(&mut self, _characterName: Option<String>, _avatarUri: Option<String>) {}

    pub fn notifyReplyCompleted(
        &mut self,
        _chatId: Option<String>,
        _characterName: Option<String>,
        _avatarUri: Option<String>,
        _notifyReplyOverride: Option<bool>,
    ) {
    }

    /// Sends a user message through the prompt, provider, and tool loop.
    pub async fn sendMessage(
        &mut self,
        options: SendMessageOptions,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let runtime = self.createSendMessageRuntime(&options)?;
        self.sendMessageWithRuntime(options, runtime).await
    }

    #[allow(non_snake_case)]
    pub fn createSendMessageRuntime(
        &mut self,
        options: &SendMessageOptions,
    ) -> Result<SendMessageRuntime, AiServiceError> {
        self.ensureInitialized();
        let (modelConfig, modelParameters, selectedService) = match (
            options.chatProviderIdOverride.as_ref(),
            options.chatModelIdOverride.as_ref(),
        ) {
            (Some(providerId), Some(modelId))
                if !providerId.trim().is_empty() && !modelId.trim().is_empty() =>
            {
                self.multi_service_manager
                    .getServiceBundleForModel(providerId.clone(), modelId.clone())?
            }
            (None, None) => self
                .multi_service_manager
                .getServiceBundleForFunction(options.functionType.clone())?,
            _ => {
                return Err(AiServiceError::RequestFailed(
                    "chat provider and model override must be set together".to_string(),
                ));
            }
        };
        let roleCardId = options.roleCardId.as_ref().ok_or_else(|| {
            AiServiceError::RequestFailed("roleCardId is required to resolve USER.md".to_string())
        })?;
        let characterPromptContext = self
            .provider_runtime_context
            .support()
            .characterPromptContext(roleCardId, options.promptFunctionType.clone())
            .map_err(AiServiceError::RequestFailed)?;
        let activeCard = &characterPromptContext.activeCard;
        let introPrompt = characterPromptContext.introPrompt;
        let aiName = characterPromptContext.aiName;
        let memoryBindingMode =
            CharacterCardMemoryBindingMode::normalize(Some(&activeCard.memoryBindingMode));
        let userOwnerKey = if memoryBindingMode == CharacterCardMemoryBindingMode::SHARED {
            let sharedMemoryId = activeCard.sharedMemoryId.as_ref().ok_or_else(|| {
                AiServiceError::RequestFailed(
                    "shared memory binding requires sharedMemoryId".to_string(),
                )
            })?;
            sharedMemoryOwnerKey(sharedMemoryId).map_err(AiServiceError::RequestFailed)?
        } else {
            characterMemoryOwnerKey(&activeCard.id).map_err(AiServiceError::RequestFailed)?
        };
        let userPreferencesText = UserMarkdownRepository::new(userOwnerKey)
            .readUserMarkdown()
            .map_err(AiServiceError::RequestFailed)?;

        Ok(SendMessageRuntime {
            activePromptMetadata: BTreeMap::new(),
            useEnglish: false,
            userPreferencesText,
            introPrompt,
            waifuRulesText: String::new(),
            avatarMoodRulesText: String::new(),
            disableUserPreferenceDescription: false,
            aiName,
            hasImageRecognition: modelConfig.capabilities.directImage,
            hasAudioRecognition: modelConfig.capabilities.directAudio,
            hasVideoRecognition: modelConfig.capabilities.directVideo,
            chatModelHasDirectAudio: modelConfig.capabilities.directAudio,
            chatModelHasDirectVideo: modelConfig.capabilities.directVideo,
            chatModelHasDirectImage: modelConfig.capabilities.directImage,
            useToolCallApi: modelConfig.capabilities.toolCall,
            toolExposureMode: match ResolvedToolExposureMode::resolve(
                modelConfig.apiProviderType.clone(),
            ) {
                ResolvedToolExposureMode::CLI => ToolExposureMode::Cli,
                ResolvedToolExposureMode::FULL => ToolExposureMode::Full,
            },
            modelConfig,
            modelParameters,
            availableTools: Vec::new(),
            aiService: selectedService,
        })
    }

    pub async fn sendMessageWithRuntime(
        &mut self,
        options: SendMessageOptions,
        runtime: SendMessageRuntime,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let eventChannel = operit_util::stream::HotStream::mutable_shared_stream(usize::MAX);
        let streamEventChannel = eventChannel.clone();
        let mut service = self.clone();
        let mut ownedOptions = Some(options);
        let mut ownedRuntime = Some(runtime);
        let coldStream = FnStream::new(move |emit| {
            let options = ownedOptions
                .take()
                .expect("sendMessageWithRuntime stream must only be collected once");
            let runtime = ownedRuntime
                .take()
                .expect("sendMessageWithRuntime runtime must only be consumed once");
            let responseStream = with_event_channel_shared(
                operit_util::stream::HotStream::mutable_shared_stream(usize::MAX),
                streamEventChannel.clone(),
            );
            let mut workerService = service.clone();
            let workerResponseStream = responseStream.clone();
            let worker = thread::spawn(move || {
                let runtimeBuilder = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime must build for EnhancedAIService stream");
                let result = runtimeBuilder.block_on(workerService.executeSendMessageWithRuntime(
                    options,
                    runtime,
                    workerResponseStream.clone(),
                ));
                workerResponseStream.upstream.close();
                workerResponseStream.event_channel.close();
                if let Err(error) = result {
                    workerService.setInputProcessingState(InputProcessingState::Error {
                        message: error.to_string(),
                    });
                }
            });
            let mut sharedCollector = responseStream.clone();
            sharedCollector.collect(emit);
            let _ = worker.join();
        });
        Ok(Box::new(
            operit_util::stream::RevisableTextStream::with_event_channel(coldStream, eventChannel),
        ))
    }

    async fn executeSendMessageWithRuntime(
        &mut self,
        options: SendMessageOptions,
        mut runtime: SendMessageRuntime,
        responseStream: SharedAiResponseStream,
    ) -> Result<(), AiServiceError> {
        let message = options.message.clone();
        let chatId = options.chatId.clone();
        let logChatId = chatId
            .clone()
            .unwrap_or_else(|| "__DEFAULT_CHAT__".to_string());
        let chatHistory = options.chatHistory.clone();
        let workspacePath = options.workspacePath.clone();
        let functionType = options.functionType.clone();
        let promptFunctionType = options.promptFunctionType.clone();
        let enableThinking = options.enableThinking;
        let enableMemoryAutoUpdate = options.enableMemoryAutoUpdate;
        let maxTokens = options.maxTokens;
        let tokenUsageThreshold = options.tokenUsageThreshold;
        let customSystemPromptTemplate = options.customSystemPromptTemplate.clone();
        let isSubTask = options.isSubTask;
        let characterName = options.characterName.clone();
        let avatarUri = options.avatarUri.clone();
        let roleCardId = options.roleCardId.clone();
        let memoryAutoUpdateCharacterCardId = roleCardId.clone();
        let enableGroupOrchestrationHint = options.enableGroupOrchestrationHint;
        let groupParticipantNamesText = options.groupParticipantNamesText.clone();
        let proxySenderName = options.proxySenderName.clone();
        let callbacks = options.callbacks;
        let notifyReplyOverride = options.notifyReplyOverride;
        let chatProviderIdOverride = options.chatProviderIdOverride.clone();
        let chatModelIdOverride = options.chatModelIdOverride.clone();
        let stream = options.stream;
        let disableWarning = options.disableWarning;
        let onNonFatalError = options.onNonFatalError;
        let onTokenLimitExceeded = options.onTokenLimitExceeded;
        let onToolInvocation = options.onToolInvocation;
        AppLogger::d(
            TAG,
            &format!(
                "sendMessage调用开始: 功能类型={}, 提示词类型={}",
                function_type_name(&functionType),
                prompt_function_type_name(&promptFunctionType)
            ),
        );

        {
            let mut shared = self.shared_state();
            shared.accumulated_input_token_count = 0;
            shared.accumulated_output_token_count = 0;
            shared.accumulated_cached_input_token_count = 0;
            shared.current_request_input_token_count = 0;
            shared.current_request_output_token_count = 0;
            shared.current_request_cached_input_token_count = 0;
        }

        let mut lifecycle = Vec::new();
        let eventChannel = MutableSharedStreamMirror::<TextStreamEventMirror>::new(usize::MAX);
        let executionId = {
            let mut shared = self.shared_state();
            shared.next_execution_context_id += 1;
            shared.next_execution_context_id
        };
        let mut execContext = MessageExecutionContext::new(
            executionId,
            chatHistory,
            workspacePath.clone(),
            eventChannel,
        );
        self.registerExecutionContext(execContext.clone());
        lifecycle.push(SendMessageLifecycleStage::EnsureInitialized);
        self.ensureInitialized();

        if !isSubTask {
            lifecycle.push(SendMessageLifecycleStage::StartAiService);
            self.startAiService(characterName.clone(), avatarUri.clone());
        }

        if !isSubTask {
            lifecycle.push(SendMessageLifecycleStage::SetProcessingState);
            self.setInputProcessingState(InputProcessingState::Processing {
                message: "enhanced_processing_message".to_string(),
            });
        }

        let runtimeSupport = self.provider_runtime_context.shared_support();
        let startTime = runtimeSupport.messageTimingNow();
        lifecycle.push(SendMessageLifecycleStage::PrepareConversationHistory);
        let preparedHistory = self.prepareConversationHistory(
            execContext.conversationHistory.clone(),
            message.clone(),
            chatId.clone(),
            workspacePath.clone(),
            promptFunctionType.clone(),
            customSystemPromptTemplate.clone(),
            roleCardId.clone(),
            enableGroupOrchestrationHint,
            groupParticipantNamesText.clone(),
            proxySenderName.clone(),
            isSubTask,
            functionType.clone(),
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            &runtime,
        );
        let tAfterPrepareHistory = runtimeSupport.messageTimingNow();
        AppLogger::d(
            TAG,
            &format!(
                "sendMessage本地耗时: prepareConversationHistory={}ms",
                tAfterPrepareHistory
                    .startedAtMs
                    .saturating_sub(startTime.startedAtMs)
            ),
        );
        lifecycle.push(SendMessageLifecycleStage::SyncPreparedHistoryToExecutionContext);
        execContext.conversationHistory.clear();
        execContext
            .conversationHistory
            .extend(preparedHistory.clone());

        if !self.isExecutionContextActive(&execContext) {
            self.unregisterExecutionContext(&execContext);
            return Ok(());
        }

        if !isSubTask {
            lifecycle.push(SendMessageLifecycleStage::SetConnectingState);
            self.setInputProcessingState(InputProcessingState::Connecting {
                message: "enhanced_connecting_service".to_string(),
            });
        }

        lifecycle.push(SendMessageLifecycleStage::GetModelParametersForFunction);
        let modelParameters = self.getModelParametersForFunction(
            functionType.clone(),
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            &runtime,
        );
        let tAfterModelParams = runtimeSupport.messageTimingNow();
        AppLogger::d(
            TAG,
            &format!(
                "sendMessage本地耗时: getModelParametersForFunction={}ms",
                tAfterModelParams
                    .startedAtMs
                    .saturating_sub(tAfterPrepareHistory.startedAtMs)
            ),
        );
        lifecycle.push(SendMessageLifecycleStage::ClearPerRequestTokenCounts);
        {
            let mut shared = self.shared_state();
            shared.per_request_token_counts = None;
            shared.current_request_input_token_count = 0;
            shared.current_request_output_token_count = 0;
            shared.current_request_cached_input_token_count = 0;
        }
        lifecycle.push(SendMessageLifecycleStage::GetAvailableToolsForFunction);
        let availableTools = self.getAvailableToolsForFunction(
            functionType.clone(),
            chatId.clone(),
            Some(promptFunctionType.clone()),
            roleCardId.clone(),
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            &runtime,
        );
        let tAfterGetTools = runtimeSupport.messageTimingNow();
        AppLogger::d(
            TAG,
            &format!(
                "sendMessage本地耗时: getAvailableToolsForFunction={}ms",
                tAfterGetTools
                    .startedAtMs
                    .saturating_sub(tAfterModelParams.startedAtMs)
            ),
        );
        lifecycle.push(SendMessageLifecycleStage::GetAIServiceForFunction);
        let serviceForFunction = self.getAIServiceForFunction(
            functionType.clone(),
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            &mut runtime,
        );
        let tAfterGetService = runtimeSupport.messageTimingNow();
        AppLogger::d(
            TAG,
            &format!(
                "sendMessage本地耗时: getAIServiceForFunction={}ms",
                tAfterGetService
                    .startedAtMs
                    .saturating_sub(tAfterGetTools.startedAtMs)
            ),
        );
        let mut finalProcessedInput = message.clone();
        let mut finalPreparedHistory = preparedHistory;
        let beforeFinalizeContext = self.applyPromptFinalizeHooks(
            PromptHookContext {
                stage: "before_finalize_prompt".to_string(),
                chat_id: chatId.clone(),
                function_type: Some(function_type_name(&functionType).to_string()),
                prompt_function_type: Some(
                    prompt_function_type_name(&promptFunctionType).to_string(),
                ),
                raw_input: Some(message.clone()),
                processed_input: Some(finalProcessedInput.clone()),
                prepared_history: finalPreparedHistory.clone(),
                model_parameters: serializePromptHookModelParameters(&modelParameters),
                available_tools: serializePromptHookToolPrompts(&availableTools),
                metadata: self.buildPromptFinalizeMetadata(
                    chatId.clone(),
                    roleCardId.clone(),
                    workspacePath.clone(),
                    enableThinking,
                    stream,
                    isSubTask,
                ),
                ..PromptHookContext::default()
            },
            PromptHookRegistry::dispatchPromptFinalizeHooks,
        );
        lifecycle.push(SendMessageLifecycleStage::BeforeFinalizePromptHook);
        if let Some(processedInput) = beforeFinalizeContext.processed_input.clone() {
            finalProcessedInput = processedInput;
        }
        finalPreparedHistory = beforeFinalizeContext.prepared_history.clone();

        let beforeSendContext = self.applyPromptFinalizeHooks(
            PromptHookContext {
                stage: "before_send_to_model".to_string(),
                processed_input: Some(finalProcessedInput.clone()),
                prepared_history: finalPreparedHistory.clone(),
                ..beforeFinalizeContext
            },
            PromptHookRegistry::dispatchPromptFinalizeHooks,
        );
        lifecycle.push(SendMessageLifecycleStage::BeforeSendToModelHook);
        if let Some(processedInput) = beforeSendContext.processed_input.clone() {
            finalProcessedInput = processedInput;
        }
        finalPreparedHistory = beforeSendContext.prepared_history.clone();

        lifecycle.push(SendMessageLifecycleStage::StripGeminiThoughtSignatureMeta);

        lifecycle.push(SendMessageLifecycleStage::ApplyFinalizedCurrentUserTurn);
        let requestHistory = self.applyFinalizedCurrentUserTurn(
            finalPreparedHistory,
            &message,
            &finalProcessedInput,
        );
        lifecycle.push(SendMessageLifecycleStage::SyncRequestHistoryToExecutionContext);
        execContext.conversationHistory.clear();
        execContext
            .conversationHistory
            .extend(requestHistory.clone());
        lifecycle.push(SendMessageLifecycleStage::EstimatePreparedRequestWindow);
        let requestWindowSize = self
            .estimatePreparedRequestWindow(
                serviceForFunction.clone(),
                &requestHistory,
                &availableTools,
                true,
            )
            .await?;
        if !self.isExecutionContextActive(&execContext) {
            self.unregisterExecutionContext(&execContext);
            return Ok(());
        }
        let tBeforeRequest = runtimeSupport.messageTimingNow();
        AppLogger::d(
            TAG,
            &format!(
                "sendMessage请求前准备耗时: {}ms, 流式输出: {}",
                tBeforeRequest
                    .startedAtMs
                    .saturating_sub(startTime.startedAtMs),
                stream
            ),
        );
        let requestStartTime = runtimeSupport.messageTimingNow();
        let _ = requestWindowSize;

        lifecycle.push(SendMessageLifecycleStage::SendMessageRequest);
        let providerModel = {
            let service = serviceForFunction.lock().await;
            service.provider_model()
        };
        AppLogger::d(
            TAG,
            &format!(
                "provider send_message begin chatId={} providerModel={}",
                logChatId, providerModel
            ),
        );
        let providerOnNonFatalError: Option<Arc<dyn Fn(String) + Send + Sync>> = {
            let callbackFn = onNonFatalError;
            let callbacks = callbacks.clone();
            if callbackFn.is_some() || callbacks.is_some() {
                Some(Arc::new(move |error: String| {
                    if let Some(callbackFn) = callbackFn {
                        callbackFn(error.clone());
                    }
                    if let Some(callbacks) = callbacks.as_ref() {
                        callbacks.onNonFatalError(error);
                    }
                }))
            } else {
                None
            }
        };
        let mut provider_stream = {
            let mut service = serviceForFunction.lock().await;
            match service
                .send_message(SendMessageRequest {
                    chat_history: requestHistory.clone(),
                    model_parameters: modelParameters.clone(),
                    enable_thinking: enableThinking,
                    stream,
                    available_tools: availableTools.clone(),
                    preserve_think_in_history: false,
                    enable_retry: true,
                    on_non_fatal_error: providerOnNonFatalError,
                    on_tool_invocation: onToolInvocation.clone(),
                })
                .await
            {
                Ok(stream) => stream,
                Err(error) => {
                    self.invalidateExecutionContext(
                        &mut execContext,
                        "sendMessage.provider.error".to_string(),
                    );
                    AppLogger::e(TAG, &format!("发送消息时发生错误: {}", error));
                    if !isSubTask {
                        self.stopAiService(characterName.clone(), avatarUri.clone());
                    }
                    self.unregisterExecutionContext(&execContext);
                    return Err(error);
                }
            }
        };
        lifecycle.push(SendMessageLifecycleStage::StartAssistantResponseRound);
        self.startAssistantResponseRound(&mut execContext);

        lifecycle.push(SendMessageLifecycleStage::CollectResponseStream);
        let providerEventChannel = provider_stream.event_channel().clone();
        let responseEventChannel = responseStream.event_channel.clone();
        let eventForwarder = thread::spawn(move || {
            let mut events = providerEventChannel;
            events.collect(&mut |event| {
                responseEventChannel.emit(event);
            });
        });
        let mut responseChunks = Vec::new();
        let mut totalChars = 0;
        let mut isFirstChunk = true;
        let mut chunkCount = 0;
        let mut lastLogTime = runtimeSupport.messageTimingNow().startedAtMs;
        let activeExecutionState = self.shared_state.clone();
        provider_stream.collect(&mut |content| {
            if !Self::isExecutionContextActiveInSharedState(&activeExecutionState, &execContext) {
                return;
            }
            if isFirstChunk {
                if !isSubTask {
                    self.setInputProcessingState(InputProcessingState::Receiving {
                        message: "enhanced_receiving_response".to_string(),
                    });
                }
                isFirstChunk = false;
                runtimeSupport.logMessageTiming(
                    "enhanced.sendMessage.firstResponseChunk",
                    requestStartTime.clone(),
                    Some(format!(
                        "functionType={}, stream={}",
                        function_type_name(&functionType),
                        stream
                    )),
                );
            }
            chunkCount += 1;
            totalChars += content.len() as i32;
            let currentTime = runtimeSupport.messageTimingNow().startedAtMs;
            if currentTime.saturating_sub(lastLogTime) > 5000 {
                AppLogger::d(
                    TAG,
                    &format!("已接收 {} 个内容块，总计 {} 个字符", chunkCount, totalChars),
                );
                lastLogTime = currentTime;
            }
            execContext.streamBuffer.push_str(&content);
            execContext.roundManager.appendContent(&content);
            responseStream.upstream.emit(content.clone());
            responseChunks.push(content);
        });
        let _ = eventForwarder.join();

        if !self.isExecutionContextActive(&execContext) {
            self.unregisterExecutionContext(&execContext);
            return Ok(());
        }

        lifecycle.push(SendMessageLifecycleStage::PersistTokenUsage);
        let (inputTokens, cachedInputTokens, outputTokens) = {
            let service = serviceForFunction.lock().await;
            (
                service.input_token_count(),
                service.cached_input_token_count(),
                service.output_token_count(),
            )
        };
        AppLogger::d(
            TAG,
            &format!(
                "provider send_message end chatId={} providerModel={} inputTokens={} outputTokens={} cachedInputTokens={} chunkCount={} totalChars={}",
                logChatId,
                providerModel,
                inputTokens,
                outputTokens,
                cachedInputTokens,
                chunkCount,
                totalChars
            ),
        );
        {
            let mut shared = self.shared_state();
            shared.accumulated_input_token_count += inputTokens;
            shared.accumulated_output_token_count += outputTokens;
            shared.accumulated_cached_input_token_count += cachedInputTokens;
            shared.current_request_input_token_count = 0;
            shared.current_request_output_token_count = 0;
            shared.current_request_cached_input_token_count = 0;
            shared.per_request_token_counts = Some((inputTokens, outputTokens));
        }
        persistProviderModelTokenUsage(
            self.provider_runtime_context.support(),
            &providerModel,
            functionType.clone(),
            UsageRequestSource::CHAT_RESPONSE,
            chatId.clone(),
            inputTokens,
            outputTokens,
            cachedInputTokens,
        )?;
        let (
            accumulatedInputTokenCount,
            accumulatedOutputTokenCount,
            accumulatedCachedInputTokenCount,
        ) = {
            let shared = self.shared_state();
            (
                shared.accumulated_input_token_count,
                shared.accumulated_output_token_count,
                shared.accumulated_cached_input_token_count,
            )
        };
        AppLogger::d(
            TAG,
            &format!(
                "Token count updated for {}. Input: {}, Output: {}, CachedInput: {}. Turn Accumulated: {}, {}, {}",
                function_type_name(&functionType),
                inputTokens,
                outputTokens,
                cachedInputTokens,
                accumulatedInputTokenCount,
                accumulatedOutputTokenCount,
                accumulatedCachedInputTokenCount
            ),
        );
        runtimeSupport.logMessageTiming(
            "enhanced.sendMessage.streamComplete",
            requestStartTime.clone(),
            Some(format!(
                "functionType={}, totalChars={}, stream={}",
                function_type_name(&functionType),
                totalChars,
                stream
            )),
        );
        let _ = totalChars;

        lifecycle.push(SendMessageLifecycleStage::ProcessStreamCompletion);
        if let Err(error) = self
            .processStreamCompletion(
                &responseStream,
                &mut execContext,
                functionType,
                promptFunctionType,
                enableThinking,
                enableMemoryAutoUpdate,
                onNonFatalError,
                onTokenLimitExceeded,
                maxTokens,
                tokenUsageThreshold,
                isSubTask,
                characterName.clone(),
                avatarUri.clone(),
                roleCardId,
                chatId.clone(),
                onToolInvocation,
                notifyReplyOverride,
                chatProviderIdOverride.clone(),
                chatModelIdOverride,
                stream,
                enableGroupOrchestrationHint,
                disableWarning,
                callbacks,
                &mut runtime,
            )
            .await
        {
            self.invalidateExecutionContext(
                &mut execContext,
                "processStreamCompletion.error".to_string(),
            );
            AppLogger::e(TAG, &format!("处理流完成时发生错误: {}", error));
            if !isSubTask {
                self.stopAiService(characterName.clone(), avatarUri.clone());
            }
            self.unregisterExecutionContext(&execContext);
            return Err(error);
        }

        if !self.isExecutionContextActive(&execContext) {
            self.unregisterExecutionContext(&execContext);
            return Ok(());
        }

        if enableMemoryAutoUpdate && !isSubTask {
            let memoryContent = execContext.roundManager.getDisplayContent();
            if !memoryContent.trim().is_empty() {
                MemoryLibrary::saveMemoryAsync(
                    promptTurnsToMemoryPairs(&requestHistory),
                    memoryContent,
                    runtime.aiService.clone(),
                    memoryAutoUpdateCharacterCardId.clone(),
                    self.provider_runtime_context.clone(),
                );
            }
        }

        lifecycle.push(SendMessageLifecycleStage::UnregisterExecutionContext);
        self.unregisterExecutionContext(&execContext);

        if !isSubTask {
            lifecycle.push(SendMessageLifecycleStage::StopAiService);
            self.stopAiService(characterName, avatarUri);
        }

        {
            let mut shared = self.shared_state();
            shared.last_reply_content = Some(execContext.roundManager.getDisplayContent());
            shared.last_provider_model = Some(providerModel);
            shared.last_turn_token_snapshot = Some(TurnTokenSnapshot {
                inputTokens,
                outputTokens,
                cachedInputTokens,
            });
        }
        let _ = finalProcessedInput;
        let _ = requestHistory;
        let _ = requestWindowSize;
        let _ = lifecycle;
        responseStream.upstream.close();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    /// Applies tool execution results to the current assistant response round.
    pub async fn processToolResults(
        &mut self,
        collector: &SharedAiResponseStream,
        results: Vec<operit_tools::ConversationMarkupManager::ToolResult>,
        context: &mut MessageExecutionContext,
        functionType: FunctionType,
        promptFunctionType: PromptFunctionType,
        enableThinking: bool,
        enableMemoryAutoUpdate: bool,
        onNonFatalError: Option<fn(String)>,
        onTokenLimitExceeded: Option<fn()>,
        maxTokens: i32,
        tokenUsageThreshold: f64,
        isSubTask: bool,
        characterName: Option<String>,
        avatarUri: Option<String>,
        roleCardId: Option<String>,
        chatId: Option<String>,
        onToolInvocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
        notifyReplyOverride: Option<bool>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        stream: bool,
        enableGroupOrchestrationHint: bool,
        toolResultMessageOverride: Option<String>,
        disableWarning: bool,
        runtime: &mut SendMessageRuntime,
    ) -> Result<(), AiServiceError> {
        let toolNames = results
            .iter()
            .map(|result| result.toolName.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let hasToolResultMessageOverride = toolResultMessageOverride.is_some();
        let rawToolResultMessage = toolResultMessageOverride
            .unwrap_or_else(|| ConversationMarkupManager::buildBoundedToolResultMessage(&results));
        let rawToolResultMessageLen = rawToolResultMessage.len();
        let toolResultMessage = rawToolResultMessage;
        let toolResultMessageLen = toolResultMessage.len();

        if toolResultMessage.trim().is_empty() {
            AppLogger::w(
                TAG,
                &format!(
                    "chat.tool_result.empty executionId={} round={} resultCount={} override={}",
                    context.executionId,
                    context.roundManager.roundIndex,
                    results.len(),
                    hasToolResultMessageOverride
                ),
            );
            return Ok(());
        }

        let displayToolNames = if toolNames.trim().is_empty() {
            "warning".to_string()
        } else {
            toolNames.clone()
        };
        let successCount = results.iter().filter(|result| result.success).count();
        AppLogger::d(
            TAG,
            &format!(
                "chat.tool_result.start executionId={} round={} resultCount={} successCount={} tools=[{}] messageChars={} override={} historyTurns={}",
                context.executionId,
                context.roundManager.roundIndex,
                results.len(),
                successCount,
                displayToolNames,
                rawToolResultMessageLen,
                hasToolResultMessageOverride,
                context.conversationHistory.len()
            ),
        );

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        if !isSubTask {
            self.setInputProcessingState(InputProcessingState::ProcessingToolResult {
                toolName: displayToolNames.clone(),
            });
        }

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        context.conversationHistory.push(PromptTurn {
            kind: PromptTurnKind::TOOL_RESULT,
            content: toolResultMessage,
            tool_name: if toolNames.trim().is_empty() {
                None
            } else {
                Some(toolNames.clone())
            },
            metadata: HashMap::new(),
        });

        let normalizedChatHistory = self
            .conversation_service
            .normalize_conversation_history_for_model(&context.conversationHistory);
        context.conversationHistory.clear();
        context.conversationHistory.extend(normalizedChatHistory);
        let currentChatHistory = context.conversationHistory.clone();

        AppLogger::d(
            TAG,
            &format!(
                "chat.tool_result.history_ready executionId={} round={} historyTurns={} messageChars={}",
                context.executionId,
                context.roundManager.roundIndex,
                currentChatHistory.len(),
                toolResultMessageLen
            ),
        );
        self.startAssistantResponseRound(context);
        AppLogger::d(
            TAG,
            &format!(
                "chat.response.round_started executionId={} round={} reason=tool_result tools=[{}]",
                context.executionId, context.roundManager.roundIndex, displayToolNames
            ),
        );

        let modelParameters = self.getModelParametersForFunction(
            functionType.clone(),
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            runtime,
        );

        let availableTools = self.getAvailableToolsForFunction(
            functionType.clone(),
            chatId.clone(),
            Some(promptFunctionType.clone()),
            roleCardId.clone(),
            chatProviderIdOverride.clone(),
            chatModelIdOverride.clone(),
            runtime,
        );

        let currentTokens = self
            .estimatePreparedRequestWindow(
                runtime.aiService.clone(),
                &currentChatHistory,
                &availableTools,
                true,
            )
            .await?;
        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        if maxTokens > 0 {
            let usageRatio = currentTokens as f64 / maxTokens as f64;
            if usageRatio >= tokenUsageThreshold {
                AppLogger::w(
                    TAG,
                    &format!(
                        "chat.token_limit executionId={} round={} currentTokens={} maxTokens={} usageRatio={:.4} threshold={:.4}",
                        context.executionId,
                        context.roundManager.roundIndex,
                        currentTokens,
                        maxTokens,
                        usageRatio,
                        tokenUsageThreshold
                    ),
                );
                if let Some(callback) = onTokenLimitExceeded {
                    callback();
                }
                self.invalidateExecutionContext(
                    context,
                    "processToolResults.tokenLimit".to_string(),
                );
                if !isSubTask {
                    self.stopAiService(characterName, avatarUri);
                }
                return Ok(());
            }
        }

        {
            let mut shared = self.shared_state();
            shared.per_request_token_counts = None;
            shared.current_request_input_token_count = 0;
            shared.current_request_output_token_count = 0;
            shared.current_request_cached_input_token_count = 0;
        }

        AppLogger::d(
            TAG,
            &format!(
                "chat.model_request.after_tool_result executionId={} round={} tools=[{}] historyTurns={} availableTools={} stream={}",
                context.executionId,
                context.roundManager.roundIndex,
                displayToolNames,
                currentChatHistory.len(),
                availableTools.len(),
                stream
            ),
        );
        let mut response = {
            let providerOnNonFatalError: Option<Arc<dyn Fn(String) + Send + Sync>> =
                onNonFatalError.map(|callbackFn| {
                    Arc::new(move |error: String| {
                        callbackFn(error);
                    }) as Arc<dyn Fn(String) + Send + Sync>
                });
            let mut service = runtime.aiService.lock().await;
            service
                .send_message(SendMessageRequest {
                    chat_history: currentChatHistory,
                    model_parameters: modelParameters,
                    enable_thinking: enableThinking,
                    stream,
                    available_tools: availableTools,
                    preserve_think_in_history: false,
                    enable_retry: true,
                    on_non_fatal_error: providerOnNonFatalError,
                    on_tool_invocation: onToolInvocation.clone(),
                })
                .await?
        };

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        if !isSubTask {
            self.setInputProcessingState(InputProcessingState::Receiving {
                message: "enhanced_receiving_tool_result".to_string(),
            });
        }

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        let responseEventChannel = response.event_channel().clone();
        let collectorEventChannel = collector.event_channel.clone();
        let eventForwarder = thread::spawn(move || {
            let mut events = responseEventChannel;
            events.collect(&mut |event| {
                collectorEventChannel.emit(event);
            });
        });
        let activeExecutionState = self.shared_state.clone();
        let mut chunkCount = 0usize;
        let mut totalChars = 0usize;
        response.collect(&mut |content| {
            if !Self::isExecutionContextActiveInSharedState(&activeExecutionState, context) {
                return;
            }
            chunkCount += 1;
            totalChars += content.len();
            context.streamBuffer.push_str(&content);
            context.roundManager.appendContent(&content);
            collector.upstream.emit(content);
        });
        let _ = eventForwarder.join();

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }
        AppLogger::d(
            TAG,
            &format!(
                "chat.model_response.after_tool_result_complete executionId={} round={} tools=[{}] chunkCount={} totalChars={}",
                context.executionId,
                context.roundManager.roundIndex,
                displayToolNames,
                chunkCount,
                totalChars
            ),
        );

        let (providerModel, inputTokens, cachedInputTokens, outputTokens) = {
            let service = runtime.aiService.lock().await;
            (
                service.provider_model(),
                service.input_token_count(),
                service.cached_input_token_count(),
                service.output_token_count(),
            )
        };
        {
            let mut shared = self.shared_state();
            shared.accumulated_input_token_count += inputTokens;
            shared.accumulated_output_token_count += outputTokens;
            shared.accumulated_cached_input_token_count += cachedInputTokens;
            shared.current_request_input_token_count = 0;
            shared.current_request_output_token_count = 0;
            shared.current_request_cached_input_token_count = 0;
            shared.per_request_token_counts = Some((inputTokens, outputTokens));
        }
        persistProviderModelTokenUsage(
            self.provider_runtime_context.support(),
            &providerModel,
            functionType.clone(),
            UsageRequestSource::TOOL_RESULT_RESPONSE,
            chatId.clone(),
            inputTokens,
            outputTokens,
            cachedInputTokens,
        )?;
        let (
            accumulatedInputTokenCount,
            accumulatedOutputTokenCount,
            accumulatedCachedInputTokenCount,
        ) = {
            let shared = self.shared_state();
            (
                shared.accumulated_input_token_count,
                shared.accumulated_output_token_count,
                shared.accumulated_cached_input_token_count,
            )
        };
        AppLogger::d(
            TAG,
            &format!(
                "Token count updated after tool result for {}. Input: {}, Output: {}, CachedInput: {}. Turn Accumulated: {}, {}, {}. executionId={}, round={}, tools=[{}]",
                function_type_name(&functionType),
                inputTokens,
                outputTokens,
                cachedInputTokens,
                accumulatedInputTokenCount,
                accumulatedOutputTokenCount,
                accumulatedCachedInputTokenCount,
                context.executionId,
                context.roundManager.roundIndex,
                displayToolNames
            ),
        );

        Box::pin(self.processStreamCompletion(
            collector,
            context,
            functionType,
            promptFunctionType,
            enableThinking,
            enableMemoryAutoUpdate,
            onNonFatalError,
            onTokenLimitExceeded,
            maxTokens,
            tokenUsageThreshold,
            isSubTask,
            characterName,
            avatarUri,
            roleCardId,
            chatId,
            onToolInvocation,
            notifyReplyOverride,
            chatProviderIdOverride,
            chatModelIdOverride,
            stream,
            enableGroupOrchestrationHint,
            disableWarning,
            None,
            runtime,
        ))
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn processStreamCompletion(
        &mut self,
        collector: &SharedAiResponseStream,
        context: &mut MessageExecutionContext,
        functionType: FunctionType,
        promptFunctionType: PromptFunctionType,
        enableThinking: bool,
        enableMemoryAutoUpdate: bool,
        onNonFatalError: Option<fn(String)>,
        onTokenLimitExceeded: Option<fn()>,
        maxTokens: i32,
        tokenUsageThreshold: f64,
        isSubTask: bool,
        characterName: Option<String>,
        avatarUri: Option<String>,
        roleCardId: Option<String>,
        chatId: Option<String>,
        onToolInvocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
        notifyReplyOverride: Option<bool>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        stream: bool,
        enableGroupOrchestrationHint: bool,
        disableWarning: bool,
        callbacks: Option<Arc<dyn SendMessageCallbacks + Send + Sync>>,
        runtime: &mut SendMessageRuntime,
    ) -> Result<(), AiServiceError> {
        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        let content = context.streamBuffer.trim().to_string();
        AppLogger::d(
            TAG,
            &format!(
                "chat.response.complete_start executionId={} round={} contentChars={} historyTurns={}",
                context.executionId,
                context.roundManager.roundIndex,
                content.len(),
                context.conversationHistory.len()
            ),
        );
        if content.is_empty() {
            AppLogger::d(
                TAG,
                &format!(
                    "chat.response.finalize_empty executionId={} round={}",
                    context.executionId, context.roundManager.roundIndex
                ),
            );
            self.finalizeAssistantResponse(
                context,
                &content,
                enableMemoryAutoUpdate,
                onNonFatalError,
                isSubTask,
                chatId.clone(),
                characterName,
                avatarUri,
                notifyReplyOverride,
                callbacks,
            );
            return Ok(());
        }

        let contentWithoutThinking = ChatUtils::remove_thinking_content(&content);
        if contentWithoutThinking.is_empty() {
            if disableWarning {
                let displayContent = context.roundManager.getDisplayContent();
                AppLogger::w(
                    TAG,
                    &format!(
                        "chat.response.finalize_pure_thinking executionId={} round={} disableWarning=true",
                        context.executionId, context.roundManager.roundIndex
                    ),
                );
                self.finalizeAssistantResponse(
                    context,
                    &displayContent,
                    enableMemoryAutoUpdate,
                    onNonFatalError,
                    isSubTask,
                    chatId.clone(),
                    characterName,
                    avatarUri,
                    notifyReplyOverride,
                    callbacks,
                );
                return Ok(());
            }
            let pureThinkingWarning =
                ConversationMarkupManager::createWarningStatus(ENHANCED_PURE_THINKING_ONLY_WARNING);
            context
                .roundManager
                .appendContent(&format!("\n{pureThinkingWarning}"));
            collector.upstream.emit(pureThinkingWarning.clone());
            context.conversationHistory.push(PromptTurn {
                kind: PromptTurnKind::TOOL_RESULT,
                content: pureThinkingWarning.clone(),
                tool_name: None,
                metadata: HashMap::new(),
            });
            return Box::pin(self.handleToolInvocation(
                collector,
                Vec::new(),
                context,
                functionType,
                promptFunctionType,
                enableThinking,
                enableMemoryAutoUpdate,
                onNonFatalError,
                onTokenLimitExceeded,
                maxTokens,
                tokenUsageThreshold,
                isSubTask,
                characterName,
                avatarUri,
                roleCardId,
                chatId,
                onToolInvocation,
                notifyReplyOverride,
                chatProviderIdOverride.clone(),
                chatModelIdOverride,
                stream,
                enableGroupOrchestrationHint,
                Some(pureThinkingWarning),
                disableWarning,
                runtime,
            ))
            .await;
        }

        let enhancedContent = self.enhanceToolDetection(&content);
        let truncatedToolRecovery = self.detectAndRepairTruncatedToolRound(&content);
        let finalContent = truncatedToolRecovery
            .as_ref()
            .map(|recovery| recovery.repairedContent.clone())
            .unwrap_or(enhancedContent);

        if let Some(recovery) = &truncatedToolRecovery {
            AppLogger::w(
                TAG,
                &format!(
                    "chat.tool_call.truncated executionId={} round={} appendedChars={} invalidatedTools={}",
                    context.executionId,
                    context.roundManager.roundIndex,
                    recovery.appendedSuffix.len(),
                    recovery.invalidatedToolNames.join(", ")
                ),
            );
            if !recovery.appendedSuffix.is_empty() {
                context.streamBuffer.push_str(&recovery.appendedSuffix);
                context.roundManager.appendContent(&recovery.appendedSuffix);
            }
        } else if finalContent != content {
            AppLogger::d(
                TAG,
                &format!(
                    "chat.tool_call.normalized_xml executionId={} round={} beforeChars={} afterChars={}",
                    context.executionId,
                    context.roundManager.roundIndex,
                    content.len(),
                    finalContent.len()
                ),
            );
            context.streamBuffer.clear();
            context.streamBuffer.push_str(&finalContent);
            context.roundManager.updateContent(finalContent.clone());
        }

        let extractedToolInvocations = if truncatedToolRecovery.is_none() {
            ToolExecutionManager::extractToolInvocations(&finalContent)
        } else {
            Vec::new()
        };
        let extractedToolNames = extractedToolInvocations
            .iter()
            .map(|invocation| invocation.tool.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        AppLogger::d(
            TAG,
            &format!(
                "chat.tool_call.detected executionId={} round={} toolCount={} tools=[{}] finalChars={}",
                context.executionId,
                context.roundManager.roundIndex,
                extractedToolInvocations.len(),
                extractedToolNames,
                finalContent.len()
            ),
        );

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        context.conversationHistory.push(PromptTurn {
            kind: PromptTurnKind::ASSISTANT,
            content: context.roundManager.getCurrentRoundContent(),
            tool_name: None,
            metadata: HashMap::new(),
        });

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        if let Some(_recovery) = truncatedToolRecovery {
            if disableWarning {
                let displayContent = context.roundManager.getDisplayContent();
                AppLogger::w(
                    TAG,
                    &format!(
                        "chat.response.finalize_truncated_tool executionId={} round={} disableWarning=true",
                        context.executionId, context.roundManager.roundIndex
                    ),
                );
                self.finalizeAssistantResponse(
                    context,
                    &displayContent,
                    enableMemoryAutoUpdate,
                    onNonFatalError,
                    isSubTask,
                    chatId.clone(),
                    characterName,
                    avatarUri,
                    notifyReplyOverride,
                    callbacks,
                );
                return Ok(());
            }
            let warningStatus = ConversationMarkupManager::createWarningStatus(
                ENHANCED_TRUNCATED_TOOL_CALL_WARNING,
            );
            let warningDisplayContent = format!("\n{warningStatus}");
            context.roundManager.appendContent(&warningDisplayContent);
            context.streamBuffer.push_str(&warningDisplayContent);
            collector.upstream.emit(warningDisplayContent);
            return Box::pin(self.handleToolInvocation(
                collector,
                Vec::new(),
                context,
                functionType,
                promptFunctionType,
                enableThinking,
                enableMemoryAutoUpdate,
                onNonFatalError,
                onTokenLimitExceeded,
                maxTokens,
                tokenUsageThreshold,
                isSubTask,
                characterName,
                avatarUri,
                roleCardId,
                chatId,
                onToolInvocation,
                notifyReplyOverride,
                chatProviderIdOverride.clone(),
                chatModelIdOverride,
                stream,
                enableGroupOrchestrationHint,
                Some(warningStatus),
                disableWarning,
                runtime,
            ))
            .await;
        }

        if !extractedToolInvocations.is_empty() {
            AppLogger::d(
                TAG,
                &format!(
                    "chat.tool_call.dispatch executionId={} round={} toolCount={} tools=[{}]",
                    context.executionId,
                    context.roundManager.roundIndex,
                    extractedToolInvocations.len(),
                    extractedToolNames
                ),
            );
            return Box::pin(self.handleToolInvocation(
                collector,
                extractedToolInvocations,
                context,
                functionType,
                promptFunctionType,
                enableThinking,
                enableMemoryAutoUpdate,
                onNonFatalError,
                onTokenLimitExceeded,
                maxTokens,
                tokenUsageThreshold,
                isSubTask,
                characterName,
                avatarUri,
                roleCardId,
                chatId,
                onToolInvocation,
                notifyReplyOverride,
                chatProviderIdOverride,
                chatModelIdOverride,
                stream,
                enableGroupOrchestrationHint,
                None,
                disableWarning,
                runtime,
            ))
            .await;
        }

        AppLogger::d(
            TAG,
            &format!(
                "chat.response.finalize_no_tools executionId={} round={} displayChars={} historyTurns={}",
                context.executionId,
                context.roundManager.roundIndex,
                context.roundManager.getDisplayContent().len(),
                context.conversationHistory.len()
            ),
        );
        self.finalizeAssistantResponse(
            context,
            &context.roundManager.getDisplayContent(),
            enableMemoryAutoUpdate,
            onNonFatalError,
            isSubTask,
            chatId.clone(),
            characterName,
            avatarUri,
            notifyReplyOverride,
            callbacks,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    /// Executes one parsed assistant tool invocation.
    pub async fn handleToolInvocation(
        &mut self,
        collector: &SharedAiResponseStream,
        toolInvocations: Vec<operit_tools::ToolExecutionManager::ToolInvocation>,
        context: &mut MessageExecutionContext,
        functionType: FunctionType,
        promptFunctionType: PromptFunctionType,
        enableThinking: bool,
        enableMemoryAutoUpdate: bool,
        onNonFatalError: Option<fn(String)>,
        onTokenLimitExceeded: Option<fn()>,
        maxTokens: i32,
        tokenUsageThreshold: f64,
        isSubTask: bool,
        characterName: Option<String>,
        avatarUri: Option<String>,
        roleCardId: Option<String>,
        chatId: Option<String>,
        onToolInvocation: Option<Arc<dyn Fn(String) + Send + Sync>>,
        notifyReplyOverride: Option<bool>,
        chatProviderIdOverride: Option<String>,
        chatModelIdOverride: Option<String>,
        stream: bool,
        enableGroupOrchestrationHint: bool,
        toolResultOverrideMessage: Option<String>,
        disableWarning: bool,
        runtime: &mut SendMessageRuntime,
    ) -> Result<(), AiServiceError> {
        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        let toolNames = toolInvocations
            .iter()
            .map(|invocation| resolveToolDisplayName(&invocation.tool))
            .collect::<Vec<_>>()
            .join(", ");
        AppLogger::d(
            TAG,
            &format!(
                "chat.tool_call.handle_start executionId={} round={} toolCount={} tools=[{}] historyTurns={}",
                context.executionId,
                context.roundManager.roundIndex,
                toolInvocations.len(),
                toolNames,
                context.conversationHistory.len()
            ),
        );

        for invocation in &toolInvocations {
            if let Some(callback) = onToolInvocation.as_ref() {
                callback(invocation.tool.name.clone());
            }
        }

        if !isSubTask && !toolInvocations.is_empty() {
            self.setInputProcessingState(InputProcessingState::ExecutingTool {
                toolName: toolNames.clone(),
            });
        }

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        self.tool_handler.registerDefaultTools();
        let packageManagerSnapshot = self
            .tool_handler
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .clone();
        let toolExposureMode = match runtime.toolExposureMode {
            ToolExposureMode::Cli => RuntimeToolExposureMode::CLI,
            ToolExposureMode::Full => RuntimeToolExposureMode::FULL,
        };
        let (emittedToolResultMessages, allToolResults) = ToolExecutionManager::executeInvocations(
            &toolInvocations,
            &mut self.tool_handler,
            &packageManagerSnapshot,
            characterName.clone(),
            chatId.clone(),
            roleCardId.clone(),
            context.workspacePath.clone(),
            toolExposureMode,
        );
        let emittedChars = emittedToolResultMessages
            .iter()
            .map(|content| content.len())
            .sum::<usize>();
        AppLogger::d(
            TAG,
            &format!(
                "chat.tool_call.handle_executed executionId={} round={} toolCount={} emittedMessages={} emittedChars={} resultCount={}",
                context.executionId,
                context.roundManager.roundIndex,
                toolInvocations.len(),
                emittedToolResultMessages.len(),
                emittedChars,
                allToolResults.len()
            ),
        );

        if !self.isExecutionContextActive(context) {
            return Ok(());
        }

        for content in emittedToolResultMessages {
            if !self.isExecutionContextActive(context) {
                return Ok(());
            }
            context.streamBuffer.push_str(&content);
            context.roundManager.appendContent(&content);
            collector.upstream.emit(content);
        }

        if !allToolResults.is_empty() {
            Box::pin(self.processToolResults(
                collector,
                allToolResults,
                context,
                functionType,
                promptFunctionType,
                enableThinking,
                enableMemoryAutoUpdate,
                onNonFatalError,
                onTokenLimitExceeded,
                maxTokens,
                tokenUsageThreshold,
                isSubTask,
                characterName,
                avatarUri,
                roleCardId,
                chatId,
                onToolInvocation,
                notifyReplyOverride,
                chatProviderIdOverride.clone(),
                chatModelIdOverride,
                stream,
                enableGroupOrchestrationHint,
                None,
                disableWarning,
                runtime,
            ))
            .await?;
        } else if match toolResultOverrideMessage.as_ref() {
            Some(value) => !value.is_empty(),
            None => false,
        } {
            Box::pin(self.processToolResults(
                collector,
                Vec::new(),
                context,
                functionType,
                promptFunctionType,
                enableThinking,
                enableMemoryAutoUpdate,
                onNonFatalError,
                onTokenLimitExceeded,
                maxTokens,
                tokenUsageThreshold,
                isSubTask,
                characterName,
                avatarUri,
                roleCardId,
                chatId,
                onToolInvocation,
                notifyReplyOverride,
                chatProviderIdOverride,
                chatModelIdOverride,
                stream,
                enableGroupOrchestrationHint,
                toolResultOverrideMessage,
                disableWarning,
                runtime,
            ))
            .await?;
        }
        Ok(())
    }

    fn enhanceToolDetection(&self, content: &str) -> String {
        if !ChatMarkupRegex::contains_tool_tag(content) {
            return content.to_string();
        }
        let mut output = String::new();
        let mut cursor = 0;
        for tool_match in ChatMarkupRegex::tool_call_matches(content) {
            output.push_str(&content[cursor..tool_match.start]);
            let xml = &content[tool_match.start..tool_match.end];
            let tagName = ChatMarkupRegex::extract_opening_tag_name(xml);
            if tagName
                .as_deref()
                .map(|name| {
                    ChatMarkupRegex::is_tool_tag_name(Some(name)) && self.isToolXmlBlock(xml, name)
                })
                .unwrap_or(false)
            {
                output.push_str(&self.normalizeToolXml(xml));
            } else {
                output.push_str(xml);
            }
            cursor = tool_match.end;
        }
        output.push_str(&content[cursor..]);
        output
    }

    fn normalizeToolXml(&self, xml: &str) -> String {
        let mut result = xml.trim().to_string();
        if let Some(toolTagName) = ChatMarkupRegex::extract_opening_tag_name(&result) {
            if ChatMarkupRegex::is_tool_tag_name(Some(&toolTagName)) {
                let tool_attr = Regex::new(&format!(
                    r#"(?i)<{}\s+name\s*="#,
                    regex::escape(&toolTagName)
                ))
                .expect("tool regex must compile");
                result = tool_attr
                    .replace_all(&result, format!("<{} name=", toolTagName))
                    .to_string();
            }
        }
        Regex::new(r#"<param\s+name\s*="#)
            .expect("param regex must compile")
            .replace_all(&result, "<param name=")
            .to_string()
    }

    fn isToolXmlBlock(&self, xml: &str, tagName: &str) -> bool {
        let trimmed = xml.trim();
        trimmed.ends_with("/>") || trimmed.contains(&format!("</{tagName}>"))
    }

    fn detectAndRepairTruncatedToolRound(
        &self,
        content: &str,
    ) -> Option<TruncatedToolRoundRecovery> {
        if !content.to_ascii_lowercase().contains("<tool") {
            return None;
        }
        let completeToolBlocks = ChatMarkupRegex::tool_call_matches(content);
        let openToolPattern = Regex::new(&format!(
            r#"(?is)<({})\b[^>]*"#,
            operit_util::ChatMarkupRegex::TOOL_TAG_NAME_REGEX_SOURCE
        ))
        .ok()?;
        let candidate = openToolPattern.find_iter(content).last()?;
        if completeToolBlocks
            .iter()
            .any(|block| candidate.start() >= block.start && candidate.end() <= block.end)
        {
            return None;
        }
        let fragment = &content[candidate.start()..];
        if !Regex::new(r#"(?i)\bname\s*=\s*""#).ok()?.is_match(fragment) {
            return None;
        }
        let tagName = ChatMarkupRegex::extract_opening_tag_name(fragment)
            .or_else(|| {
                openToolPattern
                    .captures(fragment)
                    .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
            })
            .filter(|name| ChatMarkupRegex::is_tool_tag_name(Some(name)))
            .unwrap_or_else(ChatMarkupRegex::generate_random_tool_tag_name);
        if Regex::new(&format!(r#"(?i)</{}\s*>"#, regex::escape(&tagName)))
            .ok()?
            .is_match(fragment)
        {
            return None;
        }
        let appendedSuffix = self.buildTruncatedToolRepairSuffix(fragment, &tagName);
        if appendedSuffix.is_empty() {
            return None;
        }
        let mut invalidatedToolNames = completeToolBlocks
            .iter()
            .map(|tool| tool.name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        if let Some(name) = extractXmlAttributeValue(fragment, "name") {
            if !name.trim().is_empty() {
                invalidatedToolNames.push(name.trim().to_string());
            }
        }
        invalidatedToolNames.sort();
        invalidatedToolNames.dedup();
        Some(TruncatedToolRoundRecovery {
            repairedContent: format!("{content}{appendedSuffix}"),
            appendedSuffix,
            invalidatedToolNames,
        })
    }

    fn buildTruncatedToolRepairSuffix(&self, fragment: &str, fallbackTagName: &str) -> String {
        let toolTagName = if ChatMarkupRegex::is_tool_tag_name(Some(fallbackTagName)) {
            fallbackTagName.to_string()
        } else {
            ChatMarkupRegex::generate_random_tool_tag_name()
        };
        let Some(openingTagEnd) = fragment.find('>') else {
            return format!(
                "{}</{}>",
                completePartialOpenTag(fragment, &toolTagName, "truncated_tool_call"),
                toolTagName
            );
        };
        let body = &fragment[openingTagEnd + 1..];
        let tagPattern = Regex::new(r#"(?is)</?([A-Za-z][A-Za-z0-9_]*)\b[^>]*>"#)
            .expect("tag pattern must compile");
        let mut openParamCount = 0usize;
        for capture in tagPattern.captures_iter(body) {
            let tagText = capture.get(0).map(|value| value.as_str()).unwrap_or("");
            let tagName = capture.get(1).map(|value| value.as_str()).unwrap_or("");
            let isClosing = tagText.starts_with("</");
            if tagName.eq_ignore_ascii_case("param") {
                if isClosing {
                    openParamCount = openParamCount.saturating_sub(1);
                } else if !tagText.ends_with("/>") {
                    openParamCount += 1;
                }
            } else if tagName.eq_ignore_ascii_case(&toolTagName) && isClosing {
                return String::new();
            }
        }
        let mut suffix = String::new();
        if let Some(trailingPartialTag) = extractTrailingPartialTag(fragment) {
            if isPartialClosingTagFor(&trailingPartialTag, "param") {
                suffix.push_str(&completePartialClosingTag(&trailingPartialTag, "param"));
                openParamCount = openParamCount.saturating_sub(1);
            } else if isPartialOpeningTagFor(&trailingPartialTag, "param") {
                suffix.push_str(&completePartialOpenTag(
                    &trailingPartialTag,
                    "param",
                    "_truncated_fragment",
                ));
                openParamCount += 1;
            } else if isPartialClosingTagFor(&trailingPartialTag, &toolTagName) {
                suffix.push_str(&completePartialClosingTag(
                    &trailingPartialTag,
                    &toolTagName,
                ));
                return suffix;
            } else if isPartialOpeningTagFor(&trailingPartialTag, &toolTagName) {
                suffix.push_str(&completePartialOpenTag(
                    &trailingPartialTag,
                    &toolTagName,
                    "truncated_tool_call",
                ));
            } else if trailingPartialTag == "<" {
                suffix.push_str("!-- truncated -->");
            }
        }
        for _ in 0..openParamCount {
            suffix.push_str("</param>");
        }
        suffix.push_str(&format!("</{toolTagName}>"));
        suffix
    }

    #[allow(clippy::too_many_arguments)]
    fn finalizeAssistantResponse(
        &mut self,
        context: &mut MessageExecutionContext,
        content: &str,
        _enableMemoryAutoUpdate: bool,
        _onNonFatalError: Option<fn(String)>,
        _isSubTask: bool,
        chatId: Option<String>,
        characterName: Option<String>,
        avatarUri: Option<String>,
        notifyReplyOverride: Option<bool>,
        callbacks: Option<Arc<dyn SendMessageCallbacks + Send + Sync>>,
    ) {
        self.shared_state().last_reply_content = Some(content.to_string());
        if let Some(callbacks) = callbacks {
            callbacks.onTokenLimitExceeded();
        }
        self.notifyReplyCompleted(chatId, characterName, avatarUri, notifyReplyOverride);
    }

    /// Cancels the active conversation and tool execution jobs.
    pub async fn cancelConversation(&mut self) {
        self.invalidateAllExecutionContexts("cancelConversation".to_string());
        self.multi_service_manager.cancelAllStreaming().await;
        self.input_processing_state
            .set_value(InputProcessingState::Idle);
        AppLogger::d(TAG, "Conversation canceled");
        {
            let mut shared = self.shared_state();
            shared.per_request_token_counts = None;
            shared.accumulated_input_token_count = 0;
            shared.accumulated_output_token_count = 0;
            shared.accumulated_cached_input_token_count = 0;
            shared.current_request_input_token_count = 0;
            shared.current_request_output_token_count = 0;
            shared.current_request_cached_input_token_count = 0;
            shared.current_response_callback_registered = false;
            shared.current_complete_callback_registered = false;
        }
        self.stopAiService(None, None);
        AppLogger::d(TAG, "Conversation cancellation complete");
    }

    pub fn cancelAllToolExecutions(&mut self) {
        self.shared_state().tool_execution_jobs.clear();
    }

    #[allow(non_snake_case)]
    pub fn getCurrentInputTokenCount(&self) -> i32 {
        self.shared_state().accumulated_input_token_count
    }

    #[allow(non_snake_case)]
    pub fn getCurrentOutputTokenCount(&self) -> i32 {
        self.shared_state().accumulated_output_token_count
    }

    #[allow(non_snake_case)]
    pub fn getCurrentCachedInputTokenCount(&self) -> i32 {
        self.shared_state().accumulated_cached_input_token_count
    }

    #[allow(non_snake_case)]
    pub fn getPerRequestTokenCounts(&self) -> Option<(i32, i32)> {
        self.shared_state().per_request_token_counts
    }

    #[allow(non_snake_case)]
    pub fn getRequestWindowEstimate(&self) -> Option<i32> {
        self.shared_state().request_window_estimate
    }

    #[allow(non_snake_case)]
    pub fn getLastProviderModel(&self) -> Option<String> {
        self.shared_state().last_provider_model.clone()
    }

    #[allow(non_snake_case)]
    pub fn getLastTurnTokenSnapshot(&self) -> Option<TurnTokenSnapshot> {
        self.shared_state().last_turn_token_snapshot.clone()
    }

    #[allow(non_snake_case)]
    pub fn captureCurrentTurnTokenSnapshot(&self) -> TurnTokenSnapshot {
        let shared = self.shared_state();
        TurnTokenSnapshot {
            inputTokens: (shared.accumulated_input_token_count
                + shared.current_request_input_token_count)
                .max(0),
            outputTokens: (shared.accumulated_output_token_count
                + shared.current_request_output_token_count)
                .max(0),
            cachedInputTokens: (shared.accumulated_cached_input_token_count
                + shared.current_request_cached_input_token_count)
                .max(0),
        }
    }

    #[allow(non_snake_case)]
    pub fn setCurrentTurnTokenCounts(
        &mut self,
        inputTokens: i32,
        outputTokens: i32,
        cachedInputTokens: i32,
    ) {
        let mut shared = self.shared_state();
        shared.accumulated_input_token_count = inputTokens.max(0);
        shared.accumulated_output_token_count = outputTokens.max(0);
        shared.accumulated_cached_input_token_count = cachedInputTokens.max(0);
        shared.current_request_input_token_count = 0;
        shared.current_request_output_token_count = 0;
        shared.current_request_cached_input_token_count = 0;
        shared.per_request_token_counts = Some((
            shared.accumulated_input_token_count,
            shared.accumulated_output_token_count,
        ));
    }

    #[allow(non_snake_case)]
    pub fn resetTokenCounters(&mut self) {
        let mut shared = self.shared_state();
        shared.per_request_token_counts = None;
        shared.accumulated_input_token_count = 0;
        shared.accumulated_output_token_count = 0;
        shared.accumulated_cached_input_token_count = 0;
        shared.current_request_input_token_count = 0;
        shared.current_request_output_token_count = 0;
        shared.current_request_cached_input_token_count = 0;
    }
}

impl Clone for EnhancedAIService {
    fn clone(&self) -> Self {
        Self {
            multi_service_manager: self.multi_service_manager.clone(),
            init_scope: self.init_scope.clone(),
            init_mutex: self.init_mutex.clone(),
            conversation_service: self.conversation_service.clone(),
            file_binding_service: self.file_binding_service.clone(),
            tool_handler: self.tool_handler.clone(),
            input_processing_state: self.input_processing_state.clone(),
            request_window_estimate_flow: self.request_window_estimate_flow.clone(),
            api_preferences: self.api_preferences.clone(),
            character_card_tool_access_resolver: self.character_card_tool_access_resolver.clone(),
            tool_processing_scope: self.tool_processing_scope.clone(),
            package_manager: self.package_manager.clone(),
            provider_runtime_context: self.provider_runtime_context.clone(),
            shared_state: self.shared_state.clone(),
        }
    }
}

fn apply_finalized_current_user_turn(
    preparedHistory: Vec<PromptTurn>,
    originalCurrentMessage: &str,
    finalizedCurrentMessage: &str,
) -> Vec<PromptTurn> {
    if finalizedCurrentMessage.trim().is_empty() {
        return preparedHistory;
    }

    let mut history = preparedHistory;
    if let Some(lastTurn) = history.last_mut() {
        if lastTurn.kind == PromptTurnKind::USER && lastTurn.content == finalizedCurrentMessage {
            return history;
        }
        if lastTurn.kind == PromptTurnKind::USER && lastTurn.content == originalCurrentMessage {
            lastTurn.content = finalizedCurrentMessage.to_string();
            return history;
        }
    }

    history.push(PromptTurn {
        kind: PromptTurnKind::USER,
        content: finalizedCurrentMessage.to_string(),
        tool_name: None,
        metadata: Default::default(),
    });
    history
}

#[allow(non_snake_case)]
fn persistProviderModelTokenUsage(
    runtimeSupport: &dyn ProviderRuntimeSupport,
    providerModel: &str,
    functionType: FunctionType,
    source: UsageRequestSource,
    chatId: Option<String>,
    inputTokens: i32,
    outputTokens: i32,
    cachedInputTokens: i32,
) -> Result<(), AiServiceError> {
    runtimeSupport
        .updateTokensForProviderModel(providerModel, inputTokens, outputTokens, cachedInputTokens)
        .map_err(AiServiceError::RequestFailed)?;
    UsageStatisticsStore::new()
        .recordProviderModelRequest(
            providerModel.to_string(),
            functionType,
            source,
            chatId,
            inputTokens,
            outputTokens,
            cachedInputTokens,
        )
        .map(|_| ())
        .map_err(AiServiceError::RequestFailed)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TruncatedToolRoundRecovery {
    repairedContent: String,
    appendedSuffix: String,
    invalidatedToolNames: Vec<String>,
}

fn empty_ai_response_stream() -> Box<dyn RevisableTextStreamLike> {
    response_stream_from_chunks(Vec::new())
}

fn resolveToolDisplayName(tool: &RuntimeAITool) -> String {
    if tool.name != "package_proxy" && tool.name != "proxy" {
        return tool.name.clone();
    }
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == "tool_name")
        .map(|parameter| parameter.value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| tool.name.clone())
}

#[allow(non_snake_case)]
fn buildPackageProxyToolPrompt() -> ToolPrompt {
    ToolPrompt {
        name: "package_proxy".to_string(),
        description: "Proxy tool for package tools activated by use_package.".to_string(),
        parameters: buildToolParametersJson(&[
            crate::chat::config::SystemToolPrompts::ToolParameterSchema {
                name: "tool_name".to_string(),
                value_type: "string".to_string(),
                description:
                    "Target tool name from an activated package (for example: packageName:toolName)"
                        .to_string(),
                required: true,
                default: None,
            },
            crate::chat::config::SystemToolPrompts::ToolParameterSchema {
                name: "params".to_string(),
                value_type: "object".to_string(),
                description: "JSON object of parameters to forward to the target tool".to_string(),
                required: true,
                default: None,
            },
        ]),
        parametersStructured: Some(vec![
            ToolParameterSchema {
                name: "tool_name".to_string(),
                r#type: "string".to_string(),
                description:
                    "Target tool name from an activated package (for example: packageName:toolName)"
                        .to_string(),
                required: true,
                default: None,
            },
            ToolParameterSchema {
                name: "params".to_string(),
                r#type: "object".to_string(),
                description: "JSON object of parameters to forward to the target tool".to_string(),
                required: true,
                default: None,
            },
        ]),
        details: String::new(),
        notes: String::new(),
    }
}

fn extractXmlAttributeValue(fragment: &str, name: &str) -> Option<String> {
    attr_value(fragment, name)
}

fn extractTrailingPartialTag(fragment: &str) -> Option<String> {
    let start = fragment.rfind('<')?;
    let tail = &fragment[start..];
    if tail.contains('>') {
        None
    } else {
        Some(tail.to_string())
    }
}

fn isPartialClosingTagFor(partial: &str, tagName: &str) -> bool {
    let normalized = partial.trim_start().to_ascii_lowercase();
    let target = format!("</{}", tagName.to_ascii_lowercase());
    target.starts_with(&normalized) || normalized.starts_with(&target)
}

fn isPartialOpeningTagFor(partial: &str, tagName: &str) -> bool {
    let normalized = partial.trim_start().to_ascii_lowercase();
    if normalized.starts_with("</") {
        return false;
    }
    let target = format!("<{}", tagName.to_ascii_lowercase());
    target.starts_with(&normalized) || normalized.starts_with(&target)
}

fn completePartialClosingTag(partial: &str, tagName: &str) -> String {
    let target = format!("</{tagName}>");
    completePartialToken(partial, &target)
}

fn completePartialOpenTag(partial: &str, tagName: &str, defaultNameValue: &str) -> String {
    let mut completed = partial.to_string();
    if !completed.starts_with('<') {
        completed.insert(0, '<');
    }
    if !completed
        .to_ascii_lowercase()
        .starts_with(&format!("<{}", tagName.to_ascii_lowercase()))
    {
        completed = format!("<{tagName}");
    }
    if !completed.to_ascii_lowercase().contains(" name=") {
        completed.push_str(&format!(r#" name="{defaultNameValue}""#));
    }
    if !completed.ends_with('>') {
        completed.push('>');
    }
    completed
}

fn completePartialToken(partial: &str, target: &str) -> String {
    if target.starts_with(partial) {
        target[partial.len()..].to_string()
    } else {
        target.to_string()
    }
}

fn systemToolPromptToModelToolPrompt(
    tool: crate::chat::config::SystemToolPrompts::ToolPrompt,
) -> ToolPrompt {
    ToolPrompt {
        name: tool.name,
        description: tool.description,
        parameters: buildToolParametersJson(&tool.parameters_structured),
        parametersStructured: Some(
            tool.parameters_structured
                .into_iter()
                .map(|parameter| ToolParameterSchema {
                    name: parameter.name,
                    r#type: parameter.value_type,
                    description: parameter.description,
                    required: parameter.required,
                    default: parameter.default,
                })
                .collect(),
        ),
        details: tool.details,
        notes: tool.notes,
    }
}

fn buildToolParametersJson(
    parameters: &[crate::chat::config::SystemToolPrompts::ToolParameterSchema],
) -> String {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for parameter in parameters {
        properties.insert(
            parameter.name.clone(),
            json!({
                "type": parameter.value_type,
                "description": parameter.description,
            }),
        );
        if parameter.required {
            required.push(parameter.name.clone());
        }
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
    .to_string()
}

impl From<TokenCounts> for TurnTokenSnapshot {
    fn from(value: TokenCounts) -> Self {
        Self {
            inputTokens: value.input,
            outputTokens: value.output,
            cachedInputTokens: value.cached_input,
        }
    }
}

fn serializePromptHookModelParameters(
    modelParameters: &[ModelParameter<Value>],
) -> Vec<HashMap<String, Value>> {
    modelParameters
        .iter()
        .map(|parameter| {
            HashMap::from([
                ("id".to_string(), json!(parameter.id.clone())),
                ("name".to_string(), json!(parameter.name.clone())),
                ("apiName".to_string(), json!(parameter.apiName.clone())),
                (
                    "description".to_string(),
                    json!(parameter.description.clone()),
                ),
                ("defaultValue".to_string(), parameter.defaultValue.clone()),
                ("currentValue".to_string(), parameter.currentValue.clone()),
                ("isEnabled".to_string(), json!(parameter.isEnabled)),
                (
                    "valueType".to_string(),
                    json!(format!("{:?}", parameter.valueType)),
                ),
                ("minValue".to_string(), json!(parameter.minValue.clone())),
                ("maxValue".to_string(), json!(parameter.maxValue.clone())),
                (
                    "category".to_string(),
                    json!(format!("{:?}", parameter.category)),
                ),
                ("isCustom".to_string(), json!(parameter.isCustom)),
            ])
        })
        .collect()
}

fn serializePromptHookToolPrompts(toolPrompts: &[ToolPrompt]) -> Vec<HashMap<String, Value>> {
    toolPrompts
        .iter()
        .map(|tool| {
            HashMap::from([
                ("categoryName".to_string(), json!("")),
                ("name".to_string(), json!(tool.name.clone())),
                ("description".to_string(), json!(tool.description.clone())),
                ("parameters".to_string(), json!(tool.parameters.clone())),
                ("details".to_string(), json!(tool.details.clone())),
                ("notes".to_string(), json!(tool.notes.clone())),
                (
                    "parametersStructured".to_string(),
                    json!(serializePromptHookToolParameters(
                        tool.parametersStructured.as_ref()
                    )),
                ),
            ])
        })
        .collect()
}

fn serializePromptHookToolParameters(
    parametersStructured: Option<&Vec<ToolParameterSchema>>,
) -> Vec<HashMap<String, Value>> {
    match parametersStructured {
        Some(parametersStructured) => parametersStructured
            .iter()
            .map(|parameter| {
                HashMap::from([
                    ("name".to_string(), json!(parameter.name.clone())),
                    ("type".to_string(), json!(parameter.r#type.clone())),
                    (
                        "description".to_string(),
                        json!(parameter.description.clone()),
                    ),
                    ("required".to_string(), json!(parameter.required)),
                    ("default".to_string(), json!(parameter.default.clone())),
                ])
            })
            .collect(),
        None => Vec::new(),
    }
}

fn deserializePromptHookToolPrompts(toolItems: Vec<HashMap<String, Value>>) -> Vec<ToolPrompt> {
    toolItems
        .into_iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let description = item.get("description")?.as_str()?.to_string();
            let parametersStructured =
                deserializePromptHookToolParameters(item.get("parametersStructured"));
            let parameters = item
                .get("parameters")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .expect("tool prompt parameters must be a string");
            let details = item
                .get("details")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .expect("tool prompt details must be a string");
            let notes = item
                .get("notes")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .expect("tool prompt notes must be a string");

            Some(ToolPrompt {
                name,
                description,
                parameters,
                parametersStructured: Some(parametersStructured),
                details,
                notes,
            })
        })
        .collect()
}

fn deserializePromptHookToolParameters(value: Option<&Value>) -> Vec<ToolParameterSchema> {
    match value.and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(|item| {
                let parameter = item.as_object()?;
                let name = parameter.get("name")?.as_str()?.to_string();
                let description = parameter.get("description")?.as_str()?.to_string();
                let parameter_type = parameter
                    .get("type")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .expect("tool parameter type must be a string");
                let required = parameter
                    .get("required")
                    .and_then(Value::as_bool)
                    .expect("tool parameter required must be a bool");
                let default = parameter
                    .get("default")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                Some(ToolParameterSchema {
                    name,
                    r#type: parameter_type,
                    description,
                    required,
                    default,
                })
            })
            .collect(),
        None => Vec::new(),
    }
}

fn applyToolPromptComposeHooksToAvailableTools(
    availableTools: Vec<ToolPrompt>,
    chatId: Option<String>,
    functionType: FunctionType,
    promptFunctionType: Option<PromptFunctionType>,
    useEnglish: bool,
) -> Vec<ToolPrompt> {
    let hookContext = PromptHookRegistry::dispatchToolPromptComposeHooks(PromptHookContext {
        stage: "filter_tool_call_tools".to_string(),
        chat_id: chatId,
        function_type: Some(function_type_name(&functionType).to_string()),
        prompt_function_type: promptFunctionType
            .as_ref()
            .map(prompt_function_type_name)
            .map(ToOwned::to_owned),
        use_english: Some(useEnglish),
        available_tools: serializePromptHookToolPrompts(&availableTools),
        ..PromptHookContext::default()
    });
    deserializePromptHookToolPrompts(hookContext.available_tools)
}

fn function_type_name(functionType: &FunctionType) -> &'static str {
    match functionType {
        FunctionType::CHAT => "CHAT",
        FunctionType::SUMMARY => "SUMMARY",
        FunctionType::MEMORY => "MEMORY",
        FunctionType::UI_CONTROLLER => "UI_CONTROLLER",
        FunctionType::TRANSLATION => "TRANSLATION",
        FunctionType::GREP => "GREP",
        FunctionType::ROLE_RESPONSE_PLANNER => "ROLE_RESPONSE_PLANNER",
        FunctionType::IMAGE_RECOGNITION => "IMAGE_RECOGNITION",
        FunctionType::AUDIO_RECOGNITION => "AUDIO_RECOGNITION",
        FunctionType::VIDEO_RECOGNITION => "VIDEO_RECOGNITION",
    }
}

fn prompt_function_type_name(promptFunctionType: &PromptFunctionType) -> &'static str {
    match promptFunctionType {
        PromptFunctionType::CHAT => "CHAT",
        PromptFunctionType::VOICE => "VOICE",
    }
}

fn btree_to_value_map(source: &BTreeMap<String, String>) -> HashMap<String, Value> {
    source
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect()
}

fn value_to_btree_map(source: HashMap<String, Value>) -> BTreeMap<String, String> {
    source
        .into_iter()
        .map(|(key, value)| {
            let value = match value {
                Value::String(value) => value,
                other => other.to_string(),
            };
            (key, value)
        })
        .collect()
}
