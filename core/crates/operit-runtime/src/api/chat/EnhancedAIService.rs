use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use serde_json::{json, Value};

use crate::api::chat::enhance::ConversationService::{
    ConversationService, HistoryHookContext, PrepareConversationHistoryRequest,
    PromptHistoryHookDispatcher, SystemPromptComposer, ToolExposureMode,
};
use crate::api::chat::enhance::ConversationMarkupManager::ConversationMarkupManager;
use crate::api::chat::enhance::MultiServiceManager::MultiServiceManager;
use crate::api::chat::enhance::ToolExecutionManager::ToolExecutionManager;
use crate::api::chat::llmprovider::AIService::{
    AIService, AiResponseStream, AiServiceError, SendMessageRequest, TokenCounts,
};
use crate::core::chat::hooks::PromptHookRegistry::{PromptHookContext, PromptHookRegistry};
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::core::config::SystemPromptConfig::{
    SystemPromptConfig, SystemPromptOptions, SystemPromptWithCustomOptions,
    ToolExposureMode as SystemToolExposureMode,
};
use crate::core::config::SystemToolPrompts::SystemToolPrompts;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::InputProcessingState::InputProcessingState;
use crate::data::model::ModelConfigData::ModelConfigData;
use crate::data::model::ModelParameter::ModelParameter;
use crate::data::model::PromptFunctionType::PromptFunctionType;
use crate::data::model::ToolPrompt::{ToolParameterSchema, ToolPrompt};

const TAG: &str = "EnhancedAIService";

pub struct EnhancedAIService {
    pub multi_service_manager: MultiServiceManagerMirror,
    pub init_scope: InitScopeMirror,
    pub init_mutex: InitMutexMirror,
    pub is_service_manager_initialized: bool,
    pub conversation_service: ConversationService,
    pub file_binding_service: FileBindingServiceMirror,
    pub tool_handler: AIToolHandler,
    pub input_processing_state: InputProcessingState,
    pub per_request_token_counts: Option<(i32, i32)>,
    pub request_window_estimate: Option<i32>,
    pub api_preferences: ApiPreferencesMirror,
    pub character_card_tool_access_resolver: CharacterCardToolAccessResolverMirror,
    pub active_execution_contexts: BTreeMap<i32, MessageExecutionContext>,
    pub next_execution_context_id: AtomicI32,
    pub tool_processing_scope: ToolProcessingScopeMirror,
    pub tool_execution_jobs: BTreeMap<String, ToolExecutionJobMirror>,
    pub accumulated_input_token_count: i32,
    pub accumulated_output_token_count: i32,
    pub accumulated_cached_input_token_count: i32,
    pub current_request_input_token_count: i32,
    pub current_request_output_token_count: i32,
    pub current_request_cached_input_token_count: i32,
    pub current_response_callback_registered: bool,
    pub current_complete_callback_registered: bool,
    pub package_manager: PackageManagerMirror,
    pub last_reply_content: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnTokenSnapshot {
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub cachedInputTokens: i32,
}

pub trait SendMessageCallbacks {
    fn onNonFatalError(&self, _error: String) {}

    fn onTokenLimitExceeded(&self) {}

    fn onToolInvocation(&self, _toolName: String) {}
}

pub struct SendMessageOptions<'a> {
    pub message: String,
    pub maxTokens: i32,
    pub tokenUsageThreshold: f64,
    pub chatId: Option<String>,
    pub chatHistory: Vec<PromptTurn>,
    pub workspacePath: Option<String>,
    pub workspaceEnv: Option<String>,
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
    pub callbacks: Option<&'a dyn SendMessageCallbacks>,
    pub onToolInvocation: Option<fn(String)>,
    pub notifyReplyOverride: Option<bool>,
    pub chatModelConfigIdOverride: Option<String>,
    pub chatModelIndexOverride: Option<i32>,
    pub preferenceProfileIdOverride: Option<String>,
    pub stream: bool,
    pub disableWarning: bool,
}

impl<'a> SendMessageOptions<'a> {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            maxTokens: 0,
            tokenUsageThreshold: 0.0,
            chatId: None,
            chatHistory: Vec::new(),
            workspacePath: None,
            workspaceEnv: None,
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
            chatModelConfigIdOverride: None,
            chatModelIndexOverride: None,
            preferenceProfileIdOverride: None,
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
    pub eventChannel: MutableSharedStreamMirror<TextStreamEventMirror>,
}

impl MessageExecutionContext {
    pub fn new(
        executionId: i32,
        conversationHistory: Vec<PromptTurn>,
        eventChannel: MutableSharedStreamMirror<TextStreamEventMirror>,
    ) -> Self {
        Self {
            executionId,
            streamBuffer: String::new(),
            roundManager: ConversationRoundManagerMirror::new(),
            isConversationActive: true,
            conversationHistory,
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

pub struct SendMessageRuntime<'a> {
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
    pub modelConfig: ModelConfigData,
    pub modelParameters: Vec<ModelParameter<Value>>,
    pub availableTools: Vec<ToolPrompt>,
    pub aiService: &'a mut dyn AIService,
}

#[derive(Clone, Debug)]
pub struct MultiServiceManagerMirror {
    pub initialized: bool,
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
    pub fn new() -> Self {
        Self {
            content: String::new(),
            roundIndex: 0,
        }
    }

    pub fn startNewRound(&mut self) {
        self.roundIndex += 1;
        self.content.clear();
    }

    pub fn updateContent(&mut self, content: String) {
        self.content = content;
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

pub struct RuntimeSystemPromptComposer;

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

        SystemPromptConfig::getSystemPromptWithCustomPrompts(SystemPromptWithCustomOptions {
            base: SystemPromptOptions {
                chat_id: request.chat_id.clone(),
                workspace_path: request.workspace_path.clone(),
                workspace_env: request.workspace_env.clone(),
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
    pub fn new(conversation_service: ConversationService) -> Self {
        Self {
            multi_service_manager: MultiServiceManagerMirror { initialized: false },
            init_scope: InitScopeMirror,
            init_mutex: InitMutexMirror,
            is_service_manager_initialized: false,
            conversation_service,
            file_binding_service: FileBindingServiceMirror,
            tool_handler: AIToolHandler::new(),
            input_processing_state: InputProcessingState::Idle,
            per_request_token_counts: None,
            request_window_estimate: None,
            api_preferences: ApiPreferencesMirror,
            character_card_tool_access_resolver: CharacterCardToolAccessResolverMirror,
            active_execution_contexts: BTreeMap::new(),
            next_execution_context_id: AtomicI32::new(0),
            tool_processing_scope: ToolProcessingScopeMirror,
            tool_execution_jobs: BTreeMap::new(),
            accumulated_input_token_count: 0,
            accumulated_output_token_count: 0,
            accumulated_cached_input_token_count: 0,
            current_request_input_token_count: 0,
            current_request_output_token_count: 0,
            current_request_cached_input_token_count: 0,
            current_response_callback_registered: false,
            current_complete_callback_registered: false,
            package_manager: PackageManagerMirror,
            last_reply_content: None,
        }
    }

    pub fn ensureInitialized(&mut self) {
        if self.is_service_manager_initialized {
            return;
        }
        self.multi_service_manager.initialized = true;
        self.is_service_manager_initialized = true;
    }

    pub fn getAIServiceForFunction<'a>(
        &mut self,
        _functionType: FunctionType,
        _chatModelConfigIdOverride: Option<String>,
        _chatModelIndexOverride: Option<i32>,
        runtime: &'a mut SendMessageRuntime<'_>,
    ) -> &'a mut dyn AIService {
        self.ensureInitialized();
        runtime.aiService
    }

    pub fn getProviderAndModelForFunction(&self, providerModel: &str) -> (String, String) {
        let colonIndex = providerModel.find(':').expect("providerModel must contain ':'");
        (
            providerModel[..colonIndex].to_string(),
            providerModel[colonIndex + 1..].to_string(),
        )
    }

    pub fn getModelConfigForFunction(
        &mut self,
        _functionType: FunctionType,
        _chatModelConfigIdOverride: Option<String>,
        _chatModelIndexOverride: Option<i32>,
        runtime: &SendMessageRuntime<'_>,
    ) -> ModelConfigData {
        self.ensureInitialized();
        runtime.modelConfig.clone()
    }

    pub fn refreshServiceForFunction(&mut self, _functionType: FunctionType) {
        self.ensureInitialized();
    }

    pub fn refreshAllServices(&mut self) {
        self.ensureInitialized();
    }

    pub fn getModelParametersForFunction(
        &mut self,
        _functionType: FunctionType,
        _chatModelConfigIdOverride: Option<String>,
        _chatModelIndexOverride: Option<i32>,
        runtime: &SendMessageRuntime<'_>,
    ) -> Vec<ModelParameter<Value>> {
        self.ensureInitialized();
        runtime.modelParameters.clone()
    }

    pub fn publishRequestWindowEstimate(&mut self, windowSize: i32) {
        self.request_window_estimate = Some(windowSize);
    }

    pub async fn estimatePreparedRequestWindow(
        &mut self,
        serviceForFunction: &mut dyn AIService,
        preparedHistory: &[PromptTurn],
        availableTools: &[ToolPrompt],
        publishEstimate: bool,
    ) -> Result<i32, AiServiceError> {
        let windowSize = serviceForFunction
            .calculate_input_tokens(preparedHistory, availableTools)
            .await?;
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
        workspaceEnv: Option<String>,
        enableThinking: bool,
        stream: bool,
        isSubTask: bool,
    ) -> HashMap<String, Value> {
        HashMap::from([
            ("workspacePath".to_string(), json!(workspacePath)),
            ("workspaceEnv".to_string(), json!(workspaceEnv)),
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
        workspaceEnv: Option<String>,
        promptFunctionType: PromptFunctionType,
        customSystemPromptTemplate: Option<String>,
        roleCardId: Option<String>,
        enableGroupOrchestrationHint: bool,
        groupParticipantNamesText: Option<String>,
        proxySenderName: Option<String>,
        isSubTask: bool,
        functionType: FunctionType,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
        runtime: &SendMessageRuntime<'_>,
    ) -> Vec<PromptTurn> {
        let config = self.getModelConfigForFunction(
            functionType,
            chatModelConfigIdOverride,
            chatModelIndexOverride,
            runtime,
        );
        let useToolCallApi = config.enableToolCall;
        let chatModelHasDirectImage = config.enableDirectImageProcessing;
        let chatModelHasDirectAudio = config.enableDirectAudioProcessing;
        let chatModelHasDirectVideo = config.enableDirectVideoProcessing;

        let history_hooks = RuntimePromptHistoryHooks;
        let system_prompt_composer = RuntimeSystemPromptComposer;
        self.conversation_service.prepare_conversation_history(
            PrepareConversationHistoryRequest {
                chat_history: chatHistory,
                processed_input: processedInput,
                chat_id: chatId,
                workspace_path: workspacePath,
                workspace_env: workspaceEnv,
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
                preference_profile_id_override: preferenceProfileIdOverride,
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
        let mut multiServiceManager = MultiServiceManager::default();
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
        let mut multiServiceManager = MultiServiceManager::default();
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
        _chatModelConfigIdOverride: Option<String>,
        _chatModelIndexOverride: Option<i32>,
        runtime: &SendMessageRuntime<'_>,
    ) -> Vec<ToolPrompt> {
        if !runtime.availableTools.is_empty() {
            return runtime.availableTools.clone();
        }
        if functionType != FunctionType::CHAT || !runtime.modelConfig.enableToolCall {
            return Vec::new();
        }
        self.tool_handler.registerDefaultTools();
        let categories = if runtime.useEnglish {
            SystemToolPrompts::getAIAllCategoriesEn(
                false,
                runtime.chatModelHasDirectImage,
                false,
                false,
                runtime.chatModelHasDirectAudio,
                runtime.chatModelHasDirectVideo,
                &[],
            )
        } else {
            SystemToolPrompts::getAIAllCategoriesCn(
                false,
                runtime.chatModelHasDirectImage,
                false,
                false,
                runtime.chatModelHasDirectAudio,
                runtime.chatModelHasDirectVideo,
                &[],
            )
        };
        categories
            .into_iter()
            .flat_map(|category| category.tools)
            .filter(|tool| self.tool_handler.getAllToolNames().contains(&tool.name))
            .map(systemToolPromptToModelToolPrompt)
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn estimateRequestWindowFromMemory(
        &mut self,
        message: String,
        chatHistory: Vec<PromptTurn>,
        chatId: Option<String>,
        workspacePath: Option<String>,
        workspaceEnv: Option<String>,
        promptFunctionType: PromptFunctionType,
        roleCardId: Option<String>,
        enableGroupOrchestrationHint: bool,
        groupParticipantNamesText: Option<String>,
        proxySenderName: Option<String>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
        publishEstimate: bool,
        mut runtime: SendMessageRuntime<'_>,
    ) -> Result<i32, AiServiceError> {
        self.ensureInitialized();
        let preparedHistory = self.prepareConversationHistory(
            chatHistory,
            message.clone(),
            chatId.clone(),
            workspacePath,
            workspaceEnv,
            promptFunctionType.clone(),
            None,
            roleCardId.clone(),
            enableGroupOrchestrationHint,
            groupParticipantNamesText,
            proxySenderName,
            false,
            FunctionType::CHAT,
            chatModelConfigIdOverride.clone(),
            chatModelIndexOverride,
            preferenceProfileIdOverride,
            &runtime,
        );
        let availableTools = self.getAvailableToolsForFunction(
            FunctionType::CHAT,
            chatId.clone(),
            Some(promptFunctionType.clone()),
            roleCardId,
            chatModelConfigIdOverride,
            chatModelIndexOverride,
            &runtime,
        );
        let serviceForFunction = self.getAIServiceForFunction(
            FunctionType::CHAT,
            None,
            None,
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
        self.active_execution_contexts.insert(context.executionId, context);
    }

    pub fn unregisterExecutionContext(&mut self, context: &MessageExecutionContext) {
        self.active_execution_contexts.remove(&context.executionId);
    }

    pub fn invalidateExecutionContext(
        &mut self,
        context: &mut MessageExecutionContext,
        _reason: String,
    ) {
        context.isConversationActive = false;
        if let Some(active) = self.active_execution_contexts.get_mut(&context.executionId) {
            active.isConversationActive = false;
        }
    }

    pub fn invalidateAllExecutionContexts(&mut self, reason: String) {
        let ids = self
            .active_execution_contexts
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for id in ids {
            if let Some(active) = self.active_execution_contexts.get_mut(&id) {
                active.isConversationActive = false;
            }
        }
        let _ = reason;
    }

    pub fn isExecutionContextActive(&self, context: &MessageExecutionContext) -> bool {
        context.isConversationActive
            && self
                .active_execution_contexts
                .get(&context.executionId)
                .map(|active| active.isConversationActive)
                .expect("execution context must be registered")
    }

    pub fn startAssistantResponseRound(&mut self, context: &mut MessageExecutionContext) {
        context.roundManager.startNewRound();
        context.streamBuffer.clear();
    }

    pub fn setInputProcessingState(&mut self, newState: InputProcessingState) {
        self.input_processing_state = newState;
    }

    pub fn startAiService(
        &mut self,
        _characterName: Option<String>,
        _avatarUri: Option<String>,
    ) {
    }

    pub fn stopAiService(
        &mut self,
        _characterName: Option<String>,
        _avatarUri: Option<String>,
    ) {
    }

    pub fn notifyReplyCompleted(
        &mut self,
        _chatId: Option<String>,
        _characterName: Option<String>,
        _avatarUri: Option<String>,
        _notifyReplyOverride: Option<bool>,
    ) {
    }

    pub async fn sendMessage(
        &mut self,
        options: SendMessageOptions<'_>,
    ) -> Result<SendMessageExecution, AiServiceError> {
        let mut multiServiceManager = MultiServiceManager::default();
        multiServiceManager.initialize()?;
        let modelConfig = match &options.chatModelConfigIdOverride {
            Some(configId) if !configId.trim().is_empty() => multiServiceManager
                .getModelConfigForConfig(configId.clone())?,
            _ => multiServiceManager.getModelConfigForFunction(options.functionType.clone())?,
        };
        let modelParameters = match &options.chatModelConfigIdOverride {
            Some(configId) if !configId.trim().is_empty() => multiServiceManager
                .getModelParametersForConfig(configId.clone())?,
            _ => multiServiceManager.getModelParametersForFunction(options.functionType.clone())?,
        };
        let selectedService = match &options.chatModelConfigIdOverride {
            Some(configId) if !configId.trim().is_empty() => {
                let index = match options.chatModelIndexOverride {
                    Some(value) => value,
                    None => 0,
                };
                multiServiceManager.getServiceForConfig(configId.clone(), index)?
            }
            _ => multiServiceManager.getServiceForFunction(options.functionType.clone())?,
        };
        let runtime = SendMessageRuntime {
            activePromptMetadata: BTreeMap::new(),
            useEnglish: false,
            userPreferencesText: String::new(),
            introPrompt: String::new(),
            waifuRulesText: String::new(),
            avatarMoodRulesText: String::new(),
            disableUserPreferenceDescription: false,
            aiName: "Operit".to_string(),
            hasImageRecognition: modelConfig.enableDirectImageProcessing,
            hasAudioRecognition: modelConfig.enableDirectAudioProcessing,
            hasVideoRecognition: modelConfig.enableDirectVideoProcessing,
            chatModelHasDirectAudio: modelConfig.enableDirectAudioProcessing,
            chatModelHasDirectVideo: modelConfig.enableDirectVideoProcessing,
            chatModelHasDirectImage: modelConfig.enableDirectImageProcessing,
            useToolCallApi: modelConfig.enableToolCall,
            toolExposureMode: ToolExposureMode::Cli,
            modelConfig,
            modelParameters,
            availableTools: Vec::new(),
            aiService: selectedService,
        };
        self.sendMessageWithRuntime(options, runtime).await
    }

    pub async fn sendMessageWithRuntime(
        &mut self,
        options: SendMessageOptions<'_>,
        mut runtime: SendMessageRuntime<'_>,
    ) -> Result<SendMessageExecution, AiServiceError> {
        let message = options.message.clone();
        let chatId = options.chatId.clone();
        let chatHistory = options.chatHistory.clone();
        let workspacePath = options.workspacePath.clone();
        let workspaceEnv = options.workspaceEnv.clone();
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
        let enableGroupOrchestrationHint = options.enableGroupOrchestrationHint;
        let groupParticipantNamesText = options.groupParticipantNamesText.clone();
        let proxySenderName = options.proxySenderName.clone();
        let callbacks = options.callbacks;
        let notifyReplyOverride = options.notifyReplyOverride;
        let chatModelConfigIdOverride = options.chatModelConfigIdOverride.clone();
        let chatModelIndexOverride = options.chatModelIndexOverride;
        let preferenceProfileIdOverride = options.preferenceProfileIdOverride.clone();
        let stream = options.stream;
        let disableWarning = options.disableWarning;
        let onNonFatalError = options.onNonFatalError;
        let onTokenLimitExceeded = options.onTokenLimitExceeded;
        let onToolInvocation = options.onToolInvocation;

        self.accumulated_input_token_count = 0;
        self.accumulated_output_token_count = 0;
        self.accumulated_cached_input_token_count = 0;
        self.current_request_input_token_count = 0;
        self.current_request_output_token_count = 0;
        self.current_request_cached_input_token_count = 0;

        let mut lifecycle = Vec::new();
        let eventChannel = MutableSharedStreamMirror::<TextStreamEventMirror>::new(usize::MAX);
        let mut execContext = MessageExecutionContext::new(
            self.next_execution_context_id
                .fetch_add(1, Ordering::SeqCst)
                + 1,
            chatHistory,
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

        lifecycle.push(SendMessageLifecycleStage::PrepareConversationHistory);
        let preparedHistory = self.prepareConversationHistory(
            execContext.conversationHistory.clone(),
            message.clone(),
            chatId.clone(),
            workspacePath.clone(),
            workspaceEnv.clone(),
            promptFunctionType.clone(),
            customSystemPromptTemplate.clone(),
            roleCardId.clone(),
            enableGroupOrchestrationHint,
            groupParticipantNamesText.clone(),
            proxySenderName.clone(),
            isSubTask,
            functionType.clone(),
            chatModelConfigIdOverride.clone(),
            chatModelIndexOverride,
            preferenceProfileIdOverride.clone(),
            &runtime,
        );

        lifecycle.push(SendMessageLifecycleStage::SyncPreparedHistoryToExecutionContext);
        execContext.conversationHistory.clear();
        execContext.conversationHistory.extend(preparedHistory.clone());

        if !isSubTask {
            lifecycle.push(SendMessageLifecycleStage::SetConnectingState);
            self.setInputProcessingState(InputProcessingState::Connecting {
                message: "enhanced_connecting_service".to_string(),
            });
        }

        lifecycle.push(SendMessageLifecycleStage::GetModelParametersForFunction);
        let modelParameters = self.getModelParametersForFunction(
            functionType.clone(),
            chatModelConfigIdOverride.clone(),
            chatModelIndexOverride,
            &runtime,
        );

        lifecycle.push(SendMessageLifecycleStage::ClearPerRequestTokenCounts);
        self.per_request_token_counts = None;
        self.current_request_input_token_count = 0;
        self.current_request_output_token_count = 0;
        self.current_request_cached_input_token_count = 0;

        lifecycle.push(SendMessageLifecycleStage::GetAvailableToolsForFunction);
        let availableTools = self.getAvailableToolsForFunction(
            functionType.clone(),
            chatId.clone(),
            Some(promptFunctionType.clone()),
            roleCardId.clone(),
            chatModelConfigIdOverride.clone(),
            chatModelIndexOverride,
            &runtime,
        );

        lifecycle.push(SendMessageLifecycleStage::GetAIServiceForFunction);
        let serviceForFunction = self.getAIServiceForFunction(
            functionType.clone(),
            chatModelConfigIdOverride.clone(),
            chatModelIndexOverride,
            &mut runtime,
        );

        let mut finalProcessedInput = message.clone();
        let mut finalPreparedHistory = preparedHistory;
        let beforeFinalizeContext = self.applyPromptFinalizeHooks(
            PromptHookContext {
                stage: "before_finalize_prompt".to_string(),
                chat_id: chatId.clone(),
                function_type: Some(function_type_name(&functionType).to_string()),
                prompt_function_type: Some(prompt_function_type_name(&promptFunctionType).to_string()),
                raw_input: Some(message.clone()),
                processed_input: Some(finalProcessedInput.clone()),
                prepared_history: finalPreparedHistory.clone(),
                model_parameters: serializePromptHookModelParameters(&modelParameters),
                available_tools: serializePromptHookToolPrompts(&availableTools),
                metadata: self.buildPromptFinalizeMetadata(
                    chatId.clone(),
                    roleCardId.clone(),
                    workspacePath.clone(),
                    workspaceEnv.clone(),
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
        execContext.conversationHistory.extend(requestHistory.clone());

        lifecycle.push(SendMessageLifecycleStage::EstimatePreparedRequestWindow);
        let requestWindowSize = self.estimatePreparedRequestWindow(
            serviceForFunction,
            &requestHistory,
            &availableTools,
            true,
        ).await?;

        lifecycle.push(SendMessageLifecycleStage::SendMessageRequest);
        let requestStartActive = AtomicBool::new(true);
        let providerModel = serviceForFunction.provider_model();
        let AiResponseStream {
            chunks,
            mut token_counts,
        } = serviceForFunction.send_message(SendMessageRequest {
            chat_history: requestHistory.clone(),
            model_parameters: modelParameters.clone(),
            enable_thinking: enableThinking,
            stream,
            available_tools: availableTools.clone(),
            preserve_think_in_history: false,
            enable_retry: true,
        }).await?;

        lifecycle.push(SendMessageLifecycleStage::StartAssistantResponseRound);
        self.startAssistantResponseRound(&mut execContext);

        lifecycle.push(SendMessageLifecycleStage::CollectResponseStream);
        if !isSubTask {
            self.setInputProcessingState(InputProcessingState::Receiving {
                message: "enhanced_receiving_response".to_string(),
            });
        }
        let mut responseChunks = Vec::new();
        let mut totalChars = 0;
        for content in chunks {
            totalChars += content.len() as i32;
            execContext.streamBuffer.push_str(&content);
            execContext
                .roundManager
                .updateContent(execContext.streamBuffer.clone());
            responseChunks.push(content);
        }

        lifecycle.push(SendMessageLifecycleStage::ExtractToolInvocations);
        let toolInvocations = ToolExecutionManager::extractToolInvocations(&execContext.streamBuffer);
        if !toolInvocations.is_empty() {
            lifecycle.push(SendMessageLifecycleStage::ExecuteToolInvocations);
            self.tool_handler.registerDefaultTools();
            let mut executors = self.tool_handler.takeExecutors();
            let (emittedToolResultMessages, toolResults) = ToolExecutionManager::executeInvocations(
                &toolInvocations,
                &mut executors,
                &BTreeSet::new(),
                characterName.clone(),
                chatId.clone(),
                roleCardId.clone(),
                crate::api::chat::enhance::ToolExecutionManager::ToolExposureMode::FULL,
            );
            self.tool_handler.restoreExecutors(executors);
            for content in emittedToolResultMessages {
                totalChars += content.len() as i32;
                execContext.streamBuffer.push_str(&content);
                execContext
                    .roundManager
                    .updateContent(execContext.streamBuffer.clone());
                responseChunks.push(content);
            }
            if !toolResults.is_empty() {
                lifecycle.push(SendMessageLifecycleStage::ProcessToolResults);
                let AiResponseStream {
                    chunks: followUpChunks,
                    token_counts: followUpTokenCounts,
                } = self.processToolResults(
                    toolResults,
                    &mut execContext,
                    functionType.clone(),
                    promptFunctionType.clone(),
                    enableThinking,
                    enableMemoryAutoUpdate,
                    onNonFatalError,
                    onTokenLimitExceeded,
                    maxTokens,
                    tokenUsageThreshold,
                    isSubTask,
                    characterName.clone(),
                    avatarUri.clone(),
                    roleCardId.clone(),
                    chatId.clone(),
                    onToolInvocation,
                    notifyReplyOverride,
                    chatModelConfigIdOverride.clone(),
                    chatModelIndexOverride,
                    preferenceProfileIdOverride.clone(),
                    stream,
                    enableGroupOrchestrationHint,
                    None,
                    disableWarning,
                    &mut runtime,
                ).await?;
                for content in followUpChunks {
                    totalChars += content.len() as i32;
                    execContext.streamBuffer.push_str(&content);
                    execContext
                        .roundManager
                        .updateContent(execContext.streamBuffer.clone());
                    responseChunks.push(content);
                }
                token_counts.input += followUpTokenCounts.input;
                token_counts.cached_input += followUpTokenCounts.cached_input;
                token_counts.output += followUpTokenCounts.output;
            }
        }

        lifecycle.push(SendMessageLifecycleStage::PersistTokenUsage);
        let inputTokens = token_counts.input;
        let cachedInputTokens = token_counts.cached_input;
        let outputTokens = token_counts.output;
        self.accumulated_input_token_count += inputTokens;
        self.accumulated_output_token_count += outputTokens;
        self.accumulated_cached_input_token_count += cachedInputTokens;
        self.current_request_input_token_count = 0;
        self.current_request_output_token_count = 0;
        self.current_request_cached_input_token_count = 0;
        self.per_request_token_counts = Some((inputTokens, outputTokens));
        let _ = totalChars;
        let _ = requestStartActive.load(Ordering::SeqCst);

        lifecycle.push(SendMessageLifecycleStage::ProcessStreamCompletion);
        self.processStreamCompletion(
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
            chatModelConfigIdOverride,
            chatModelIndexOverride,
            preferenceProfileIdOverride,
            stream,
            enableGroupOrchestrationHint,
            disableWarning,
            callbacks,
        );

        lifecycle.push(SendMessageLifecycleStage::UnregisterExecutionContext);
        self.unregisterExecutionContext(&execContext);

        if !isSubTask {
            lifecycle.push(SendMessageLifecycleStage::StopAiService);
            self.stopAiService(characterName, avatarUri);
        }

        Ok(SendMessageExecution {
            processedInput: finalProcessedInput,
            requestHistory,
            responseChunks,
            tokenSnapshot: TurnTokenSnapshot {
                inputTokens,
                outputTokens,
                cachedInputTokens,
            },
            requestWindowSize,
            providerModel,
            lifecycle,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn processToolResults(
        &mut self,
        results: Vec<crate::api::chat::enhance::ConversationMarkupManager::ToolResult>,
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
        onToolInvocation: Option<fn(String)>,
        notifyReplyOverride: Option<bool>,
        chatModelConfigIdOverride: Option<String>,
        chatModelIndexOverride: Option<i32>,
        preferenceProfileIdOverride: Option<String>,
        stream: bool,
        enableGroupOrchestrationHint: bool,
        toolResultMessageOverride: Option<String>,
        disableWarning: bool,
        runtime: &mut SendMessageRuntime<'_>,
    ) -> Result<AiResponseStream, AiServiceError> {
        let toolNames = results
            .iter()
            .map(|result| result.toolName.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let rawToolResultMessage = toolResultMessageOverride
            .unwrap_or_else(|| ConversationMarkupManager::buildBoundedToolResultMessage(&results));
        let toolResultMessage = rawToolResultMessage;

        if toolResultMessage.trim().is_empty() {
            return Ok(AiResponseStream {
                chunks: Vec::new(),
                token_counts: TokenCounts {
                    input: 0,
                    cached_input: 0,
                    output: 0,
                },
            });
        }

        let displayToolNames = if toolNames.trim().is_empty() {
            "warning".to_string()
        } else {
            toolNames.clone()
        };

        if !isSubTask {
            self.setInputProcessingState(InputProcessingState::ProcessingToolResult {
                toolName: displayToolNames.clone(),
            });
        }

        if !context.isConversationActive {
            return Ok(AiResponseStream {
                chunks: Vec::new(),
                token_counts: TokenCounts {
                    input: 0,
                    cached_input: 0,
                    output: 0,
                },
            });
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

        self.startAssistantResponseRound(context);

        if !isSubTask {
            self.setInputProcessingState(InputProcessingState::ProcessingToolResult {
                toolName: displayToolNames.clone(),
            });
        }

        let modelParameters = self.getModelParametersForFunction(
            functionType.clone(),
            chatModelConfigIdOverride.clone(),
            chatModelIndexOverride,
            runtime,
        );

        let availableTools = self.getAvailableToolsForFunction(
            functionType.clone(),
            chatId.clone(),
            Some(promptFunctionType.clone()),
            roleCardId.clone(),
            chatModelConfigIdOverride.clone(),
            chatModelIndexOverride,
            runtime,
        );

        let currentTokens = self
            .estimatePreparedRequestWindow(
                runtime.aiService,
                &currentChatHistory,
                &availableTools,
                true,
            )
            .await?;

        if maxTokens > 0 {
            let usageRatio = currentTokens as f64 / maxTokens as f64;
            if usageRatio >= tokenUsageThreshold {
                if let Some(callback) = onTokenLimitExceeded {
                    callback();
                }
                context.isConversationActive = false;
                if !isSubTask {
                    self.stopAiService(characterName, avatarUri);
                }
                return Ok(AiResponseStream {
                    chunks: Vec::new(),
                    token_counts: TokenCounts {
                        input: 0,
                        cached_input: 0,
                        output: 0,
                    },
                });
            }
        }

        self.per_request_token_counts = None;
        self.current_request_input_token_count = 0;
        self.current_request_output_token_count = 0;
        self.current_request_cached_input_token_count = 0;

        let response = runtime
            .aiService
            .send_message(SendMessageRequest {
                chat_history: currentChatHistory,
                model_parameters: modelParameters,
                enable_thinking: enableThinking,
                stream,
                available_tools: availableTools,
                preserve_think_in_history: false,
                enable_retry: true,
            })
            .await?;

        if !isSubTask {
            self.setInputProcessingState(InputProcessingState::Receiving {
                message: "enhanced_receiving_tool_result".to_string(),
            });
        }

        self.processStreamCompletion(
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
            chatModelConfigIdOverride,
            chatModelIndexOverride,
            preferenceProfileIdOverride,
            stream,
            enableGroupOrchestrationHint,
            disableWarning,
            None,
        );

        Ok(response)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn processStreamCompletion(
        &mut self,
        context: &mut MessageExecutionContext,
        _functionType: FunctionType,
        _promptFunctionType: PromptFunctionType,
        _enableThinking: bool,
        _enableMemoryAutoUpdate: bool,
        _onNonFatalError: Option<fn(String)>,
        _onTokenLimitExceeded: Option<fn()>,
        _maxTokens: i32,
        _tokenUsageThreshold: f64,
        _isSubTask: bool,
        characterName: Option<String>,
        avatarUri: Option<String>,
        _roleCardId: Option<String>,
        chatId: Option<String>,
        _onToolInvocation: Option<fn(String)>,
        notifyReplyOverride: Option<bool>,
        _chatModelConfigIdOverride: Option<String>,
        _chatModelIndexOverride: Option<i32>,
        _preferenceProfileIdOverride: Option<String>,
        _stream: bool,
        _enableGroupOrchestrationHint: bool,
        _disableWarning: bool,
        callbacks: Option<&dyn SendMessageCallbacks>,
    ) {
        self.last_reply_content = Some(context.streamBuffer.clone());
        if let Some(callbacks) = callbacks {
            callbacks.onTokenLimitExceeded();
        }
        self.notifyReplyCompleted(chatId, characterName, avatarUri, notifyReplyOverride);
    }

    pub fn cancelConversation(&mut self, service: &mut dyn AIService) {
        self.invalidateAllExecutionContexts("cancelConversation".to_string());
        service.cancel_streaming();
        self.input_processing_state = InputProcessingState::Idle;
        self.per_request_token_counts = None;
        self.accumulated_input_token_count = 0;
        self.accumulated_output_token_count = 0;
        self.accumulated_cached_input_token_count = 0;
        self.current_request_input_token_count = 0;
        self.current_request_output_token_count = 0;
        self.current_request_cached_input_token_count = 0;
        self.current_response_callback_registered = false;
        self.current_complete_callback_registered = false;
        self.stopAiService(None, None);
    }

    pub fn cancelAllToolExecutions(&mut self) {
        self.tool_execution_jobs.clear();
    }

    #[allow(non_snake_case)]
    pub fn getCurrentInputTokenCount(&self) -> i32 {
        self.accumulated_input_token_count
    }

    #[allow(non_snake_case)]
    pub fn getCurrentOutputTokenCount(&self) -> i32 {
        self.accumulated_output_token_count
    }

    #[allow(non_snake_case)]
    pub fn getCurrentCachedInputTokenCount(&self) -> i32 {
        self.accumulated_cached_input_token_count
    }

    #[allow(non_snake_case)]
    pub fn captureCurrentTurnTokenSnapshot(&self) -> TurnTokenSnapshot {
        TurnTokenSnapshot {
            inputTokens: (self.accumulated_input_token_count + self.current_request_input_token_count).max(0),
            outputTokens: (self.accumulated_output_token_count + self.current_request_output_token_count).max(0),
            cachedInputTokens: (self.accumulated_cached_input_token_count
                + self.current_request_cached_input_token_count)
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
        self.accumulated_input_token_count = inputTokens.max(0);
        self.accumulated_output_token_count = outputTokens.max(0);
        self.accumulated_cached_input_token_count = cachedInputTokens.max(0);
        self.current_request_input_token_count = 0;
        self.current_request_output_token_count = 0;
        self.current_request_cached_input_token_count = 0;
        self.per_request_token_counts = Some((
            self.accumulated_input_token_count,
            self.accumulated_output_token_count,
        ));
    }

    #[allow(non_snake_case)]
    pub fn resetTokenCounters(&mut self) {
        self.per_request_token_counts = None;
        self.accumulated_input_token_count = 0;
        self.accumulated_output_token_count = 0;
        self.accumulated_cached_input_token_count = 0;
        self.current_request_input_token_count = 0;
        self.current_request_output_token_count = 0;
        self.current_request_cached_input_token_count = 0;
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

fn systemToolPromptToModelToolPrompt(
    tool: crate::core::config::SystemToolPrompts::ToolPrompt,
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
    parameters: &[crate::core::config::SystemToolPrompts::ToolParameterSchema],
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
                ("description".to_string(), json!(parameter.description.clone())),
                ("defaultValue".to_string(), parameter.defaultValue.clone()),
                ("currentValue".to_string(), parameter.currentValue.clone()),
                ("isEnabled".to_string(), json!(parameter.isEnabled)),
                ("valueType".to_string(), json!(format!("{:?}", parameter.valueType))),
                ("minValue".to_string(), json!(parameter.minValue.clone())),
                ("maxValue".to_string(), json!(parameter.maxValue.clone())),
                ("category".to_string(), json!(format!("{:?}", parameter.category))),
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
                    ("description".to_string(), json!(parameter.description.clone())),
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
