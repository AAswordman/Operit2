use std::collections::{BTreeMap, HashMap};

use regex::Regex;
use serde_json::{json, Value};

use crate::api::chat::enhance::MultiServiceManager::MultiServiceManager;
use crate::api::chat::llmprovider::AIService::{
    collect_stream_chunks, AiServiceError, SendMessageRequest,
};
use crate::core::chat::hooks::SummaryHookRegistry::{
    SummaryHookContext, SummaryHookRegistry,
};
use crate::core::chat::hooks::PromptTurn::{PromptTurn, PromptTurnKind};
use crate::core::config::FunctionalPrompts::FunctionalPrompts;
use crate::data::model::FunctionType::FunctionType;
use crate::data::model::ModelParameter::ModelParameter;
use crate::util::ChatMarkupRegex::{attr_value, ChatMarkupRegex};

const APPLY_FILE_TOOL_NAME: &str = "apply_file";

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

#[derive(Clone, Debug, Default)]
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
            let text = &content[cursor..open_start];
            if !text.trim().is_empty() {
                tags.push(vec!["text".to_string(), text.to_string()]);
            }
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
                let end = body_end + close_tag.len();
                tags.push(vec![tag_name, content[open_start..end].to_string()]);
                cursor = end;
            } else {
                cursor = open_end + 1;
            }
        }
        let tail = &content[cursor..];
        if !tail.trim().is_empty() {
            tags.push(vec!["text".to_string(), tail.to_string()]);
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
        xml_tags: &[Vec<String>],
        prepared_history: &mut Vec<PromptTurn>,
        _index: usize,
        _history_size: usize,
    ) {
        if xml_tags.is_empty() {
            prepared_history.push(PromptTurn {
                kind: PromptTurnKind::ASSISTANT,
                content: content.to_string(),
                tool_name: None,
                metadata: Default::default(),
            });
            return;
        }

        let mut segments = Vec::new();
        for tag in xml_tags {
            let tag_name = tag.get(0).cloned().unwrap_or_default();
            let normalized_tag_name = ChatMarkupRegex::normalize_tool_like_tag_name(Some(&tag_name))
                .unwrap_or_else(|| tag_name.clone());
            let tag_content = tag.get(1).cloned().unwrap_or_default();

            match normalized_tag_name.as_str() {
                "text" => {
                    if !tag_content.trim().is_empty() {
                        segments.push(PromptTurn::new(PromptTurnKind::ASSISTANT, tag_content));
                    }
                }
                "think" | "thinking" => {
                    segments.push(PromptTurn::new(PromptTurnKind::ASSISTANT, tag_content));
                }
                "status" => {
                    let kind = if tag_content.contains("type=\"complete\"")
                        || tag_content.contains("type=\"wait_for_user_need\"")
                    {
                        PromptTurnKind::ASSISTANT
                    } else {
                        PromptTurnKind::USER
                    };
                    segments.push(PromptTurn::new(kind, tag_content));
                }
                "tool_result" => {
                    segments.push(PromptTurn::new(
                        PromptTurnKind::TOOL_RESULT,
                        self.normalize_tool_result_markup_for_model(&tag_content),
                    ));
                }
                "tool" => {
                    segments.push(PromptTurn::new(PromptTurnKind::TOOL_CALL, tag_content));
                }
                _ => {
                    segments.push(PromptTurn::new(PromptTurnKind::ASSISTANT, tag_content));
                }
            }
        }

        let mut merged_segments = Vec::new();
        let mut current_kind: Option<PromptTurnKind> = None;
        let mut current_content = String::new();
        let mut current_tool_name: Option<String> = None;
        let mut current_metadata: HashMap<String, Value> = HashMap::new();

        for segment in segments {
            let should_merge_current = current_kind
                .as_ref()
                .map(|kind| {
                    *kind == segment.kind
                        && segment.kind != PromptTurnKind::TOOL_CALL
                        && segment.kind != PromptTurnKind::TOOL_RESULT
                })
                .unwrap_or(false);
            if should_merge_current {
                current_content.push('\n');
                current_content.push_str(&segment.content);
            } else {
                if !current_content.is_empty() {
                    if let Some(kind) = current_kind.clone() {
                        merged_segments.push(PromptTurn {
                            kind,
                            content: current_content.trim().to_string(),
                            tool_name: current_tool_name.clone(),
                            metadata: current_metadata.clone(),
                        });
                    }
                    current_content.clear();
                }
                current_kind = Some(segment.kind.clone());
                current_tool_name = segment.tool_name.clone();
                current_metadata = segment.metadata.clone();
                current_content.push_str(&segment.content);
            }
        }

        if !current_content.is_empty() {
            if let Some(kind) = current_kind {
                merged_segments.push(PromptTurn {
                    kind,
                    content: current_content.trim().to_string(),
                    tool_name: current_tool_name,
                    metadata: current_metadata,
                });
            }
        }

        prepared_history.extend(merged_segments);
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

        let summaryStream = summaryService
            .send_message(SendMessageRequest {
                chat_history: preparedHistory.clone(),
                model_parameters: modelParameters,
                enable_thinking: false,
                stream: true,
                available_tools: Vec::new(),
                preserve_think_in_history: false,
                enable_retry: true,
                on_tool_invocation: None,
            })
            .await?;
        let summaryChunks = collect_stream_chunks(summaryStream);
        let mut summaryContent =
            removeThinkingContent(&summaryChunks.join("").trim().to_string());

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
                    metadata.insert(
                        "inputTokens".to_string(),
                        json!(summaryService.input_token_count()),
                    );
                    metadata.insert(
                        "cachedInputTokens".to_string(),
                        json!(summaryService.cached_input_token_count()),
                    );
                    metadata.insert(
                        "outputTokens".to_string(),
                        json!(summaryService.output_token_count()),
                    );
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
        let blocks = ChatMarkupRegex::tool_result_blocks(content);
        if blocks.is_empty() {
            return content.to_string();
        }

        let mut normalized = String::new();
        let mut cursor = 0usize;
        for block in blocks {
            normalized.push_str(&content[cursor..block.start]);
            let tool_name = attr_value(&block.raw, "name").unwrap_or_default();
            if !tool_name.eq_ignore_ascii_case(APPLY_FILE_TOOL_NAME) {
                normalized.push_str(&block.raw);
                cursor = block.end;
                continue;
            }

            let Some(request_content) = self.extract_apply_file_request_content(&block.body) else {
                normalized.push_str(&block.raw);
                cursor = block.end;
                continue;
            };

            let opening_end = block.raw.find('>').unwrap_or(block.raw.len());
            let opening_tag = &block.raw[..=opening_end];
            normalized.push_str(opening_tag);
            normalized.push_str("<content>");
            normalized.push_str(&request_content);
            normalized.push_str("</content></");
            normalized.push_str(&block.tag_name);
            normalized.push('>');
            cursor = block.end;
        }
        normalized.push_str(&content[cursor..]);
        normalized
    }

    fn extract_apply_file_request_content(&self, tool_result_body: &str) -> Option<String> {
        let content_body = extract_xml_tag_body_case_insensitive(tool_result_body, "content")
            .unwrap_or(tool_result_body);
        let file_request_content_regex = Regex::new(
            r#"(?is)<file-request-content\b[^>]*><!\[CDATA\[(.*?)\]\]></file-request-content>"#,
        )
        .expect("file request content regex must compile");
        file_request_content_regex
            .captures(content_body)
            .and_then(|captures| captures.get(1).map(|value| value.as_str().trim().to_string()))
    }
}

fn extract_xml_tag_body_case_insensitive<'a>(content: &'a str, tag_name: &str) -> Option<&'a str> {
    let lower = content.to_ascii_lowercase();
    let open_prefix = format!("<{}", tag_name.to_ascii_lowercase());
    let open_start = lower.find(&open_prefix)?;
    let open_end = lower[open_start..].find('>')? + open_start;
    let close_tag = format!("</{}>", tag_name.to_ascii_lowercase());
    let body_start = open_end + 1;
    let close_start = lower[body_start..].find(&close_tag)? + body_start;
    Some(&content[body_start..close_start])
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
