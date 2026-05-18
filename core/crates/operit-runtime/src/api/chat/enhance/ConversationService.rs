use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::api::chat::enhance::MultiServiceManager::MultiServiceManager;
use crate::api::chat::llmprovider::AIService::{
    AiResponseStream, AiServiceError, SendMessageRequest,
};
use crate::core::chat::hooks::SummaryHookRegistry::{
    SummaryHookContext, SummaryHookRegistry,
};
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::core::config::FunctionalPrompts::FunctionalPrompts;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::ModelParameter::ModelParameter;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolExposureMode {
    Full,
    Cli,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrepareConversationHistoryRequest {
    pub chat_history: Vec<PromptTurn>,
    pub processed_input: String,
    pub chat_id: Option<String>,
    pub workspace_path: Option<String>,
    pub workspace_env: Option<String>,
    pub prompt_function_type: String,
    pub custom_system_prompt_template: Option<String>,
    pub role_card_id: Option<String>,
    pub enable_group_orchestration_hint: bool,
    pub group_participant_names_text: Option<String>,
    pub proxy_sender_name: Option<String>,
    pub has_image_recognition: bool,
    pub has_audio_recognition: bool,
    pub has_video_recognition: bool,
    pub chat_model_has_direct_audio: bool,
    pub chat_model_has_direct_video: bool,
    pub use_tool_call_api: bool,
    pub chat_model_has_direct_image: bool,
    pub tool_exposure_mode: ToolExposureMode,
    pub preference_profile_id_override: Option<String>,
    pub active_prompt_metadata: BTreeMap<String, String>,
    pub user_preferences_text: String,
    pub intro_prompt: String,
    pub waifu_rules_text: String,
    pub avatar_mood_rules_text: String,
    pub disable_user_preference_description: bool,
    pub ai_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryHookContext {
    pub stage: String,
    pub chat_id: Option<String>,
    pub prompt_function_type: String,
    pub processed_input: String,
    pub chat_history: Vec<PromptTurn>,
    pub prepared_history: Vec<PromptTurn>,
    pub use_english: Option<bool>,
    pub metadata: BTreeMap<String, String>,
}

pub trait PromptHistoryHookDispatcher {
    fn dispatch_prompt_history_hooks(&self, context: HistoryHookContext) -> HistoryHookContext;
}

pub trait SystemPromptComposer {
    fn get_system_prompt_with_custom_prompts(
        &self,
        request: &PrepareConversationHistoryRequest,
        use_english: bool,
    ) -> String;
}

pub struct ConversationService;

impl ConversationService {
    pub fn prepare_conversation_history(
        &self,
        request: PrepareConversationHistoryRequest,
        history_hooks: &dyn PromptHistoryHookDispatcher,
        system_prompt_composer: &dyn SystemPromptComposer,
        use_english: bool,
    ) -> Vec<PromptTurn> {
        let before_context = history_hooks.dispatch_prompt_history_hooks(HistoryHookContext {
            stage: "before_prepare_history".to_string(),
            chat_id: request.chat_id.clone(),
            prompt_function_type: request.prompt_function_type.clone(),
            processed_input: request.processed_input.clone(),
            chat_history: request.chat_history.clone(),
            prepared_history: Vec::new(),
            use_english: None,
            metadata: build_prepare_history_metadata(&request),
        });

        let effective_chat_history = before_context.chat_history.clone();
        let mut prepared_history = Vec::new();

        if !effective_chat_history
            .iter()
            .any(|turn| turn.kind == PromptTurnKind::SYSTEM)
        {
            let system_prompt = system_prompt_composer
                .get_system_prompt_with_custom_prompts(&request, use_english);
            let final_system_prompt = build_final_system_prompt(
                &request.avatar_mood_rules_text,
                &system_prompt,
                &request.waifu_rules_text,
                &request.user_preferences_text,
                request.disable_user_preference_description,
            );
            prepared_history.push(PromptTurn {
                kind: PromptTurnKind::SYSTEM,
                content: replace_prompt_placeholders(&final_system_prompt, &request.ai_name),
                tool_name: None,
                metadata: Default::default(),
            });
        }

        for (index, turn) in effective_chat_history.iter().enumerate() {
            match turn.kind {
                PromptTurnKind::ASSISTANT => {
                    let xml_tags = self.split_xml_tag(&turn.content);
                    if xml_tags.is_empty() {
                        prepared_history.push(turn.clone());
                    } else {
                        self.process_chat_message_with_tools(
                            &turn.content,
                            &xml_tags,
                            &mut prepared_history,
                            index,
                            effective_chat_history.len(),
                        );
                    }
                }
                PromptTurnKind::TOOL_RESULT => {
                    prepared_history.push(PromptTurn {
                        kind: PromptTurnKind::TOOL_RESULT,
                        content: self.normalize_tool_result_markup_for_model(&turn.content),
                        tool_name: turn.tool_name.clone(),
                        metadata: turn.metadata.clone(),
                    });
                }
                _ => prepared_history.push(turn.clone()),
            }
        }

        let after_context = history_hooks.dispatch_prompt_history_hooks(HistoryHookContext {
            stage: "after_prepare_history".to_string(),
            prepared_history,
            use_english: Some(use_english),
            ..before_context
        });
        after_context.prepared_history
    }

    pub fn split_xml_tag(&self, content: &str) -> Vec<Vec<String>> {
        let mut tags = Vec::new();
        let mut cursor = 0;
        while let Some(open_offset) = content[cursor..].find('<') {
            let open_start = cursor + open_offset;
            let open_end = match content[open_start..].find('>') {
                Some(value) => open_start + value,
                None => break,
            };
            let tag_name = content[open_start + 1..open_end]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_start_matches('/')
                .to_string();
            if tag_name.is_empty() {
                cursor = open_end + 1;
                continue;
            }
            let close_tag = format!("</{}>", tag_name);
            let body_start = open_end + 1;
            if let Some(close_offset) = content[body_start..].find(&close_tag) {
                let body_end = body_start + close_offset;
                tags.push(vec![tag_name, content[body_start..body_end].to_string()]);
                cursor = body_end + close_tag.len();
            } else {
                cursor = open_end + 1;
            }
        }
        tags
    }

    pub fn normalize_conversation_history_for_model(
        &self,
        chat_history: &[PromptTurn],
    ) -> Vec<PromptTurn> {
        chat_history
            .iter()
            .map(|turn| match turn.kind {
                PromptTurnKind::ASSISTANT | PromptTurnKind::TOOL_CALL | PromptTurnKind::TOOL_RESULT => {
                    PromptTurn {
                        kind: turn.kind.clone(),
                        content: self.normalize_tool_result_markup_for_model(&turn.content),
                        tool_name: turn.tool_name.clone(),
                        metadata: turn.metadata.clone(),
                    }
                }
                _ => turn.clone(),
            })
            .collect()
    }

    pub fn process_chat_message_with_tools(
        &self,
        content: &str,
        _xml_tags: &[Vec<String>],
        prepared_history: &mut Vec<PromptTurn>,
        _index: usize,
        _history_size: usize,
    ) {
        prepared_history.push(PromptTurn {
            kind: PromptTurnKind::ASSISTANT,
            content: content.to_string(),
            tool_name: None,
            metadata: Default::default(),
        });
    }

    pub fn build_preferences_text(&self, profile_items: &[(String, String)]) -> String {
        profile_items
            .iter()
            .map(|(key, value)| format!("{}: {}", key, value))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub async fn generateSummary(
        &self,
        messages: Vec<(String, String)>,
        previousSummary: Option<String>,
        multiServiceManager: &mut MultiServiceManager,
    ) -> Result<String, AiServiceError> {
        self.generateSummaryFromPromptTurns(
            messages
                .into_iter()
                .map(|(role, content)| PromptTurn::new(PromptTurnKind::from_role(&role), content))
                .collect(),
            previousSummary,
            multiServiceManager,
        )
        .await
    }

    pub async fn generateSummaryFromPromptTurns(
        &self,
        messages: Vec<PromptTurn>,
        previousSummary: Option<String>,
        multiServiceManager: &mut MultiServiceManager,
    ) -> Result<String, AiServiceError> {
        let useEnglish = false;
        let mut systemPrompt =
            FunctionalPrompts::buildSummarySystemPrompt(previousSummary.as_deref(), useEnglish);
        let modelParameters =
            multiServiceManager.getModelParametersForFunction(FunctionType::SUMMARY)?;
        let serializedModelParameters = serializeSummaryHookModelParameters(&modelParameters);
        let summaryService = multiServiceManager.getServiceForFunction(FunctionType::SUMMARY)?;
        let mut summaryHistory = stripGeminiThoughtSignatureMetaTurns(messages);
        let mut summaryPrompt = FunctionalPrompts::summaryUserMessage(useEnglish).to_string();
        let baseSummaryMetadata = std::collections::HashMap::from([
            ("providerModel".to_string(), json!(summaryService.provider_model())),
            ("sourceMessageCount".to_string(), json!(summaryHistory.len())),
        ]);

        let beforePrepareContext = SummaryHookRegistry::dispatchSummaryGenerateHooks(
            SummaryHookContext {
                stage: "before_prepare_summary_prompt".to_string(),
                use_english: Some(useEnglish),
                previous_summary: previousSummary.clone(),
                chat_history: summaryHistory,
                prepared_history: Vec::new(),
                system_prompt: Some(systemPrompt),
                summary_prompt: Some(summaryPrompt),
                summary_result: None,
                model_parameters: serializedModelParameters.clone(),
                metadata: baseSummaryMetadata.clone(),
            },
        );
        summaryHistory = beforePrepareContext.chat_history;
        systemPrompt = beforePrepareContext
            .system_prompt
            .expect("SummaryHookContext.system_prompt must be present after before_prepare_summary_prompt");
        summaryPrompt = beforePrepareContext
            .summary_prompt
            .expect("SummaryHookContext.summary_prompt must be present after before_prepare_summary_prompt");
        let mut preparedHistory = if beforePrepareContext.prepared_history.is_empty() {
            buildSummaryPreparedHistory(
                systemPrompt.clone(),
                summaryHistory.clone(),
                summaryPrompt.clone(),
            )
        } else {
            beforePrepareContext.prepared_history
        };

        let beforeSendBasePreparedHistory = preparedHistory.clone();
        let beforeSendContext = SummaryHookRegistry::dispatchSummaryGenerateHooks(
            SummaryHookContext {
                stage: "before_send_to_model".to_string(),
                use_english: Some(useEnglish),
                previous_summary: previousSummary.clone(),
                chat_history: summaryHistory,
                prepared_history: preparedHistory.clone(),
                system_prompt: Some(systemPrompt),
                summary_prompt: Some(summaryPrompt),
                summary_result: None,
                model_parameters: serializedModelParameters.clone(),
                metadata: {
                    let mut metadata = baseSummaryMetadata.clone();
                    metadata.insert(
                        "preparedMessageCount".to_string(),
                        json!(preparedHistory.len()),
                    );
                    metadata
                },
            },
        );
        summaryHistory = beforeSendContext.chat_history;
        systemPrompt = beforeSendContext
            .system_prompt
            .expect("SummaryHookContext.system_prompt must be present after before_send_to_model");
        summaryPrompt = beforeSendContext
            .summary_prompt
            .expect("SummaryHookContext.summary_prompt must be present after before_send_to_model");
        preparedHistory = if beforeSendContext.prepared_history != beforeSendBasePreparedHistory {
            beforeSendContext.prepared_history
        } else {
            buildSummaryPreparedHistory(
                systemPrompt.clone(),
                summaryHistory.clone(),
                summaryPrompt.clone(),
            )
        };

        let AiResponseStream {
            chunks,
            token_counts,
        } = summaryService
            .send_message(SendMessageRequest {
                chat_history: preparedHistory.clone(),
                model_parameters: modelParameters,
                enable_thinking: false,
                stream: true,
                available_tools: Vec::new(),
                preserve_think_in_history: false,
                enable_retry: true,
            })
            .await?;
        let mut summaryContent = removeThinkingContent(&chunks.join("").trim().to_string());

        let afterGenerateContext = SummaryHookRegistry::dispatchSummaryGenerateHooks(
            SummaryHookContext {
                stage: "after_generate_summary".to_string(),
                use_english: Some(useEnglish),
                previous_summary: previousSummary,
                chat_history: summaryHistory,
                prepared_history: preparedHistory.clone(),
                system_prompt: Some(systemPrompt),
                summary_prompt: Some(summaryPrompt),
                summary_result: Some(summaryContent.clone()),
                model_parameters: serializedModelParameters,
                metadata: {
                    let mut metadata = baseSummaryMetadata;
                    metadata.insert(
                        "preparedMessageCount".to_string(),
                        json!(preparedHistory.len()),
                    );
                    metadata.insert("inputTokens".to_string(), json!(token_counts.input));
                    metadata.insert(
                        "cachedInputTokens".to_string(),
                        json!(token_counts.cached_input),
                    );
                    metadata.insert("outputTokens".to_string(), json!(token_counts.output));
                    metadata
                },
            },
        );
        summaryContent = afterGenerateContext
            .summary_result
            .expect("SummaryHookContext.summary_result must be present after after_generate_summary");
        if summaryContent.trim().is_empty() {
            return Ok("Conversation Summary: Unable to generate valid summary.".to_string());
        }
        Ok(summaryContent)
    }

    pub fn translate_text(&self, text: &str) -> String {
        text.to_string()
    }

    pub fn generate_package_description(&self, plugin_name: &str, tool_descriptions: &[String]) -> String {
        format!("{}\n{}", plugin_name, tool_descriptions.join("\n"))
    }

    pub fn analyze_image_with_intent(&self, image_path: &str, user_intent: Option<&str>) -> String {
        build_media_intent_prompt("image", image_path, user_intent)
    }

    pub fn analyze_audio_with_intent(&self, audio_path: &str, user_intent: Option<&str>) -> String {
        build_media_intent_prompt("audio", audio_path, user_intent)
    }

    pub fn analyze_video_with_intent(&self, video_path: &str, user_intent: Option<&str>) -> String {
        build_media_intent_prompt("video", video_path, user_intent)
    }

    fn normalize_tool_result_markup_for_model(&self, content: &str) -> String {
        content.replace("<tool_result", "<tool_result").replace("</tool_result>", "</tool_result>")
    }
}

fn build_prepare_history_metadata(
    request: &PrepareConversationHistoryRequest,
) -> BTreeMap<String, String> {
    let mut metadata = request.active_prompt_metadata.clone();
    insert_option(&mut metadata, "workspacePath", request.workspace_path.as_ref());
    insert_option(&mut metadata, "workspaceEnv", request.workspace_env.as_ref());
    insert_option(
        &mut metadata,
        "customSystemPromptTemplate",
        request.custom_system_prompt_template.as_ref(),
    );
    metadata.insert(
        "enableGroupOrchestrationHint".to_string(),
        request.enable_group_orchestration_hint.to_string(),
    );
    insert_option(
        &mut metadata,
        "groupParticipantNamesText",
        request.group_participant_names_text.as_ref(),
    );
    insert_option(&mut metadata, "proxySenderName", request.proxy_sender_name.as_ref());
    metadata.insert(
        "hasImageRecognition".to_string(),
        request.has_image_recognition.to_string(),
    );
    metadata.insert(
        "hasAudioRecognition".to_string(),
        request.has_audio_recognition.to_string(),
    );
    metadata.insert(
        "hasVideoRecognition".to_string(),
        request.has_video_recognition.to_string(),
    );
    metadata.insert(
        "chatModelHasDirectAudio".to_string(),
        request.chat_model_has_direct_audio.to_string(),
    );
    metadata.insert(
        "chatModelHasDirectVideo".to_string(),
        request.chat_model_has_direct_video.to_string(),
    );
    metadata.insert("useToolCallApi".to_string(), request.use_tool_call_api.to_string());
    metadata.insert(
        "chatModelHasDirectImage".to_string(),
        request.chat_model_has_direct_image.to_string(),
    );
    metadata.insert(
        "toolExposureMode".to_string(),
        format!("{:?}", request.tool_exposure_mode),
    );
    metadata
}

fn insert_option(target: &mut BTreeMap<String, String>, key: &str, value: Option<&String>) {
    if let Some(value) = value {
        target.insert(key.to_string(), value.clone());
    }
}

#[allow(non_snake_case)]
fn buildSummaryPreparedHistory(
    systemPrompt: String,
    chatHistory: Vec<PromptTurn>,
    summaryPrompt: String,
) -> Vec<PromptTurn> {
    let mut prepared = Vec::with_capacity(chatHistory.len() + 2);
    prepared.push(PromptTurn::new(PromptTurnKind::SYSTEM, systemPrompt));
    prepared.extend(chatHistory);
    prepared.push(PromptTurn::new(PromptTurnKind::USER, summaryPrompt));
    prepared
}

#[allow(non_snake_case)]
fn serializeSummaryHookModelParameters(
    modelParameters: &[ModelParameter<Value>],
) -> Vec<std::collections::HashMap<String, Value>> {
    modelParameters
        .iter()
        .map(|parameter| {
            std::collections::HashMap::from([
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

#[allow(non_snake_case)]
fn stripGeminiThoughtSignatureMetaTurns(messages: Vec<PromptTurn>) -> Vec<PromptTurn> {
    messages
}

#[allow(non_snake_case)]
fn removeThinkingContent(input: &str) -> String {
    let mut remaining = input.to_string();
    loop {
        let Some(start) = remaining.find("<think>") else {
            break;
        };
        let Some(end_relative) = remaining[start + "<think>".len()..].find("</think>") else {
            break;
        };
        let end = start + "<think>".len() + end_relative + "</think>".len();
        remaining.replace_range(start..end, " ");
    }
    remaining.trim().to_string()
}

fn build_final_system_prompt(
    avatar_mood_rules_text: &str,
    system_prompt: &str,
    waifu_rules_text: &str,
    preferences_text: &str,
    disable_user_preference_description: bool,
) -> String {
    let mut final_prompt = String::new();
    final_prompt.push_str(avatar_mood_rules_text);
    final_prompt.push_str(system_prompt);
    final_prompt.push_str(waifu_rules_text);
    if !disable_user_preference_description && !preferences_text.is_empty() {
        final_prompt.push_str("\n\nUser preference description: ");
        final_prompt.push_str(preferences_text);
    }
    final_prompt
}

fn replace_prompt_placeholders(prompt: &str, ai_name: &str) -> String {
    prompt.replace("{{aiName}}", ai_name).replace("{aiName}", ai_name)
}

fn build_media_intent_prompt(media_type: &str, path: &str, user_intent: Option<&str>) -> String {
    match user_intent {
        Some(intent) => format!("{}:{}\n{}", media_type, path, intent),
        None => format!("{}:{}", media_type, path),
    }
}
