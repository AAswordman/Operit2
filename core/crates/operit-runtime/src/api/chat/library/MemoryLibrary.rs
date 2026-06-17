use std::collections::HashMap;
use std::sync::OnceLock;
use std::thread;

use chrono::Utc;
use regex::Regex;
use serde_json::Value;

use crate::api::chat::enhance::MultiServiceManager::SharedAIServiceHandle;
use crate::api::chat::llmprovider::AIService::SendMessageRequest;
use crate::core::chat::hooks::PromptTurn::{toPromptTurns, PromptTurn, PromptTurnKind};
use crate::core::config::FunctionalPrompts::FunctionalPrompts;
use crate::data::model::Memory::{Memory, MemoryTag};
use crate::data::preferences::MemorySearchSettingsPreferences::MemorySearchSettingsPreferences;
use crate::data::repository::MemoryRepository::MemoryRepository;
use crate::data::repository::UserMarkdownRepository::UserMarkdownRepository;
use crate::util::stream::Stream::Stream;
use crate::util::AppLogger::AppLogger;
use crate::util::ChatMarkupRegex::{tag_ranges, ChatMarkupRegex};
use crate::util::ChatUtils::ChatUtils;
use crate::util::OperitPaths::characterMemoryOwnerKey;

const TAG: &str = "MemoryLibrary";

pub struct MemoryLibrary;

#[derive(Clone, Debug)]
struct ParsedLink {
    sourceTitle: String,
    targetTitle: String,
    type_: String,
    description: String,
    weight: f32,
}

#[derive(Clone, Debug)]
struct ParsedEntity {
    title: String,
    content: String,
    tags: Vec<String>,
    aliasFor: Option<String>,
    folderPath: Option<String>,
}

#[derive(Clone, Debug)]
struct ParsedUpdate {
    titleToUpdate: String,
    newContent: String,
    reason: String,
    newCredibility: Option<f32>,
    newImportance: Option<f32>,
}

#[derive(Clone, Debug)]
struct ParsedMerge {
    sourceTitles: Vec<String>,
    newTitle: String,
    newContent: String,
    newTags: Vec<String>,
    folderPath: String,
    reason: String,
}

#[derive(Clone, Debug)]
struct ParsedAnalysis {
    mainProblem: Option<ParsedEntity>,
    extractedEntities: Vec<ParsedEntity>,
    links: Vec<ParsedLink>,
    updatedEntities: Vec<ParsedUpdate>,
    mergedEntities: Vec<ParsedMerge>,
    userPreferences: String,
}

impl ParsedAnalysis {
    fn empty() -> Self {
        Self {
            mainProblem: None,
            extractedEntities: Vec::new(),
            links: Vec::new(),
            updatedEntities: Vec::new(),
            mergedEntities: Vec::new(),
            userPreferences: String::new(),
        }
    }
}

impl MemoryLibrary {
    #[allow(non_snake_case)]
    pub fn saveMemoryAsync(
        conversationHistory: Vec<(String, String)>,
        content: String,
        aiService: SharedAIServiceHandle,
        characterCardId: Option<String>,
    ) {
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime must build for MemoryLibrary");
            let result = runtime.block_on(Self::saveMemoryNow(
                conversationHistory,
                content,
                aiService,
                characterCardId,
            ));
            if let Err(error) = result {
                AppLogger::e(TAG, &format!("保存记忆失败: {error}"));
            }
        });
    }

    #[allow(non_snake_case)]
    pub async fn saveMemoryNow(
        conversationHistory: Vec<(String, String)>,
        content: String,
        aiService: SharedAIServiceHandle,
        characterCardId: Option<String>,
    ) -> Result<(), String> {
        Self::saveMemory(conversationHistory, content, aiService, characterCardId).await
    }

    #[allow(non_snake_case)]
    async fn saveMemory(
        conversationHistory: Vec<(String, String)>,
        content: String,
        aiService: SharedAIServiceHandle,
        characterCardId: Option<String>,
    ) -> Result<(), String> {
        let mutex = memoryMutex();
        let _guard = mutex.lock().await;
        let characterCardId = characterCardId
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "characterCardId is required for memory auto update".to_string())?;
        let ownerKey = characterMemoryOwnerKey(&characterCardId)?;
        let memoryRepository = MemoryRepository::new(ownerKey.clone());
        let prunedContent =
            ChatUtils::strip_gemini_thought_signature_meta(&pruneToolResultContent(&content));

        let processedHistory = conversationHistory
            .into_iter()
            .filter(|(role, _)| role != "system")
            .map(|(role, msgContent)| {
                let cleanedContent = if role == "user" {
                    removeMemoryTags(&msgContent).trim().to_string()
                } else {
                    msgContent
                };
                (
                    role,
                    ChatUtils::strip_gemini_thought_signature_meta(&pruneToolResultContent(
                        &cleanedContent,
                    )),
                )
            })
            .collect::<Vec<_>>();

        if processedHistory.is_empty() {
            return Ok(());
        }
        let Some((_, query)) = processedHistory
            .iter()
            .rev()
            .find(|(role, _)| role == "user")
        else {
            return Ok(());
        };
        if query.is_empty() {
            return Ok(());
        }

        let analysis = Self::generateAnalysis(
            aiService,
            query,
            &prunedContent,
            &processedHistory,
            &memoryRepository,
            &ownerKey,
        )
        .await?;

        if analysis.mainProblem.is_none()
            && analysis.extractedEntities.is_empty()
            && analysis.updatedEntities.is_empty()
            && analysis.mergedEntities.is_empty()
        {
            return Ok(());
        }

        let mut createdMemories = HashMap::<String, Memory>::new();

        for merge in &analysis.mergedEntities {
            let _ = &merge.reason;
            if let Some(mergedMemory) = memoryRepository.mergeMemories(
                merge.sourceTitles.clone(),
                merge.newTitle.clone(),
                merge.newContent.clone(),
                merge.newTags.clone(),
                merge.folderPath.clone(),
            )? {
                createdMemories.insert(mergedMemory.title.clone(), mergedMemory);
            }
        }

        for update in &analysis.updatedEntities {
            let _ = &update.reason;
            if let Some(memoryToUpdate) =
                memoryRepository.findMemoryByTitle(&update.titleToUpdate)?
            {
                let updatedMemory = memoryRepository.updateMemory(
                    memoryToUpdate.id,
                    memoryToUpdate.title,
                    update.newContent.clone(),
                    memoryToUpdate.contentType,
                    memoryToUpdate.source,
                    update.newCredibility.unwrap_or(memoryToUpdate.credibility),
                    update.newImportance.unwrap_or(memoryToUpdate.importance),
                    memoryToUpdate.folderPath,
                    Some(
                        memoryToUpdate
                            .tags
                            .into_iter()
                            .map(|tag| tag.name)
                            .collect(),
                    ),
                )?;
                createdMemories.insert(updatedMemory.title.clone(), updatedMemory);
            }
        }

        if !analysis.userPreferences.is_empty() {
            UserMarkdownRepository::new(&ownerKey)
                .writeUserMarkdown(analysis.userPreferences.clone())?;
        }

        let Some(mainProblem) = analysis.mainProblem.clone() else {
            return Ok(());
        };

        let mainProblemMemory = if let Some(mut existingMemory) =
            memoryRepository.findMemoryByTitle(&mainProblem.title)?
        {
            existingMemory.content = mainProblem.content.clone();
            memoryRepository.saveMemory(existingMemory)?
        } else {
            let mut memory = newMemory(
                mainProblem.title.clone(),
                mainProblem.content.clone(),
                "memory_analysis".to_string(),
                mainProblem.folderPath.clone(),
                1.0,
                0.8,
            );
            memory.tags = buildTags(mainProblem.tags.clone());
            memoryRepository.saveMemory(memory)?
        };
        createdMemories.insert(mainProblemMemory.title.clone(), mainProblemMemory);

        for entity in &analysis.extractedEntities {
            let mut memory = None;
            if let Some(aliasFor) = entity
                .aliasFor
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                memory = createdMemories
                    .get(aliasFor)
                    .cloned()
                    .or(memoryRepository.findMemoryByTitle(aliasFor)?);
            }
            if memory.is_none() {
                let mut created = newMemory(
                    entity.title.clone(),
                    entity.content.clone(),
                    "memory_analysis".to_string(),
                    entity
                        .folderPath
                        .clone()
                        .or_else(|| mainProblem.folderPath.clone()),
                    0.5,
                    0.5,
                );
                created.tags = buildTags(entity.tags.clone());
                memory = Some(memoryRepository.saveMemory(created)?);
            }
            if let Some(memory) = memory {
                createdMemories.insert(entity.title.clone(), memory);
            }
        }

        for link in &analysis.links {
            let source = match createdMemories.get(&link.sourceTitle).cloned() {
                Some(memory) => Some(memory),
                None => memoryRepository.findMemoryByTitle(&link.sourceTitle)?,
            };
            let target = match createdMemories.get(&link.targetTitle).cloned() {
                Some(memory) => Some(memory),
                None => memoryRepository.findMemoryByTitle(&link.targetTitle)?,
            };
            if let (Some(source), Some(target)) = (source, target) {
                memoryRepository.linkMemories(
                    source.id,
                    target.id,
                    link.type_.clone(),
                    link.weight,
                    link.description.clone(),
                )?;
            }
        }

        Ok(())
    }

    #[allow(non_snake_case)]
    async fn generateAnalysis(
        aiService: SharedAIServiceHandle,
        query: &str,
        solution: &str,
        conversationHistory: &[(String, String)],
        memoryRepository: &MemoryRepository,
        ownerKey: &str,
    ) -> Result<ParsedAnalysis, String> {
        let useEnglish = false;
        let currentPreferences = UserMarkdownRepository::new(ownerKey).readUserMarkdown()?;
        let contextQuery = buildCandidateSearchQuery(query, solution);
        let searchConfig = MemorySearchSettingsPreferences::new(ownerKey)
            .load()
            .map_err(|error| error.to_string())?;
        let candidateMemories = memoryRepository
            .searchMemories(&contextQuery, None, 0.0, None, None)?
            .into_iter()
            .take(15)
            .collect::<Vec<_>>();
        let duplicatesPromptPart =
            findAndDescribeDuplicates(&candidateMemories, memoryRepository, useEnglish)?;
        let existingMemoriesPrompt = if candidateMemories.is_empty() {
            FunctionalPrompts::knowledgeGraphNoExistingMemoriesMessage(useEnglish).to_string()
        } else {
            format!(
                "{}{}",
                FunctionalPrompts::knowledgeGraphExistingMemoriesPrefix(useEnglish),
                candidateMemories
                    .iter()
                    .map(|memory| {
                        format!(
                            "- \"{}\": {}...",
                            memory.title,
                            memory
                                .content
                                .replace('\n', " ")
                                .chars()
                                .take(150)
                                .collect::<String>()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        let existingFoldersPrompt = FunctionalPrompts::knowledgeGraphExistingFoldersPrompt(
            &memoryRepository.getAllFolderPaths()?,
            useEnglish,
        );
        let systemPrompt = FunctionalPrompts::buildKnowledgeGraphExtractionPrompt(
            &duplicatesPromptPart,
            &existingMemoriesPrompt,
            &existingFoldersPrompt,
            &currentPreferences,
            useEnglish,
        );
        let analysisMessage =
            buildAnalysisMessage(query, solution, conversationHistory, useEnglish);
        let messages = toPromptTurns(&[
            ("system".to_string(), systemPrompt),
            ("user".to_string(), analysisMessage),
        ]);
        let mut result = String::new();
        {
            let mut service = aiService.lock().await;
            let mut stream = service
                .send_message(SendMessageRequest {
                    chat_history: messages,
                    model_parameters: Vec::new(),
                    enable_thinking: false,
                    stream: true,
                    available_tools: Vec::new(),
                    preserve_think_in_history: false,
                    enable_retry: true,
                    on_non_fatal_error: None,
                    on_tool_invocation: None,
                })
                .await
                .map_err(|error| error.to_string())?;
            stream.collect(&mut |chunk| {
                result.push_str(&chunk);
            });
        }
        let _ = searchConfig;
        parseAnalysisResult(&ChatUtils::remove_thinking_content(&result), useEnglish)
    }
}

#[allow(non_snake_case)]
pub fn promptTurnsToMemoryPairs(turns: &[PromptTurn]) -> Vec<(String, String)> {
    turns
        .iter()
        .map(|turn| (turn.role().to_string(), turn.content.clone()))
        .collect()
}

fn memoryMutex() -> &'static tokio::sync::Mutex<()> {
    static MEMORY_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    MEMORY_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[allow(non_snake_case)]
fn buildCandidateSearchQuery(query: &str, solution: &str) -> String {
    let coreQuestion = extractCoreQuestionText(query);
    let selectedQuestion = if coreQuestion.trim().is_empty() {
        normalizeCandidateSearchText(query, 800)
    } else {
        coreQuestion
    };
    if selectedQuestion.trim().is_empty() {
        return normalizeCandidateSearchText(solution, 300);
    }
    let conciseSolution = normalizeCandidateSearchText(solution, 180);
    if conciseSolution.trim().is_empty() {
        selectedQuestion
    } else {
        format!("{selectedQuestion}\n{conciseSolution}")
    }
}

#[allow(non_snake_case)]
fn extractCoreQuestionText(rawQuery: &str) -> String {
    let compact = rawQuery.replace("\r\n", "\n");
    let cn = Regex::new(r"(?s)问题\s*[：:]\s*(.+?)(?:\n\s*解决方案\s*[：:]|\z)")
        .expect("memory regex must compile")
        .captures(&compact)
        .and_then(|captures| {
            captures
                .get(1)
                .map(|value| value.as_str().trim().to_string())
        });
    let en = Regex::new(r"(?s)Question\s*:\s*(.+?)(?:\n\s*Solution\s*:|\z)")
        .expect("memory regex must compile")
        .captures(&compact)
        .and_then(|captures| {
            captures
                .get(1)
                .map(|value| value.as_str().trim().to_string())
        });
    let selected = cn.or(en).unwrap_or(compact);
    let filtered = selected
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("历史记录:") && !trimmed.starts_with("History:")
        })
        .collect::<Vec<_>>()
        .join("\n");
    normalizeCandidateSearchText(&filtered, 500)
}

#[allow(non_snake_case)]
fn normalizeCandidateSearchText(raw: &str, maxLen: usize) -> String {
    let mut text = raw.to_string();
    for pattern in [
        r"(?is)<tool(?:_[A-Za-z0-9_]+)?\b[^>]*>.*?</tool(?:_[A-Za-z0-9_]+)?>",
        r"(?is)<tool(?:_[A-Za-z0-9_]+)?\b[^>]*/>",
        r"(?is)<tool_result(?:_[A-Za-z0-9_]+)?\b[^>]*>.*?</tool_result(?:_[A-Za-z0-9_]+)?>",
        r"(?is)<tool_result(?:_[A-Za-z0-9_]+)?\b[^>]*/>",
        r"(?is)<status\b[^>]*>.*?</status>",
        r"(?is)<status\b[^>]*/>",
        r"(?is)<think(?:ing)?\b[^>]*>.*?</think(?:ing)?>",
        r"(?is)<think(?:ing)?\b[^>]*/>",
        r"(?is)<search\b[^>]*>.*?</search>",
        r"(?is)<search\b[^>]*/>",
        r"https?://\S+",
        r"[`*_#>]+",
        r"\s+",
    ] {
        text = Regex::new(pattern)
            .expect("memory cleanup regex must compile")
            .replace_all(&text, " ")
            .to_string();
    }
    text.trim().chars().take(maxLen).collect()
}

#[allow(non_snake_case)]
fn findAndDescribeDuplicates(
    candidateMemories: &[Memory],
    memoryRepository: &MemoryRepository,
    useEnglish: bool,
) -> Result<String, String> {
    let mut titles = candidateMemories
        .iter()
        .map(|memory| memory.title.clone())
        .collect::<Vec<_>>();
    titles.sort();
    titles.dedup();
    let mut duplicatesFound = Vec::new();
    for title in titles {
        let memoriesWithSameTitle = memoryRepository.findMemoriesByTitle(&title)?;
        if memoriesWithSameTitle.len() > 1 {
            duplicatesFound.push(FunctionalPrompts::knowledgeGraphDuplicateTitleInstruction(
                &title,
                memoriesWithSameTitle.len(),
                useEnglish,
            ));
        }
    }
    if duplicatesFound.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!(
            "{}{}\n",
            FunctionalPrompts::knowledgeGraphDuplicateHeader(useEnglish),
            duplicatesFound.join("\n")
        ))
    }
}

#[allow(non_snake_case)]
fn buildAnalysisMessage(
    query: &str,
    solution: &str,
    conversationHistory: &[(String, String)],
    useEnglish: bool,
) -> String {
    let mut message = String::new();
    if useEnglish {
        message.push_str("Question:\n");
        message.push_str(query);
        message.push_str("\n\nSolution:\n");
        message.push_str(&solution.chars().take(3000).collect::<String>());
        message.push_str("\n\n");
    } else {
        message.push_str("问题：\n");
        message.push_str(query);
        message.push_str("\n\n解决方案：\n");
        message.push_str(&solution.chars().take(3000).collect::<String>());
        message.push_str("\n\n");
    }
    let recentHistory = conversationHistory
        .iter()
        .rev()
        .take(10)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    if !recentHistory.is_empty() {
        message.push_str(if useEnglish {
            "History:\n"
        } else {
            "历史记录：\n"
        });
        for (index, (role, content)) in recentHistory.iter().enumerate() {
            message.push_str(&format!(
                "#{} {}: {}\n",
                index + 1,
                role,
                content.chars().take(4000).collect::<String>()
            ));
        }
    }
    message
}

#[allow(non_snake_case)]
fn parseAnalysisResult(jsonString: &str, _useEnglish: bool) -> Result<ParsedAnalysis, String> {
    let cleanJson = ChatUtils::extract_json(jsonString);
    if cleanJson.trim().is_empty()
        || !cleanJson.trim_start().starts_with('{')
        || cleanJson.trim() == "{}"
    {
        return Ok(ParsedAnalysis::empty());
    }
    let json: Value = serde_json::from_str(&cleanJson).map_err(|error| error.to_string())?;
    let mainProblem = json.get("main").and_then(parseEntityArray);
    let extractedEntities = json
        .get("new")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(parseEntityArray)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let links = json
        .get("links")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(parseLinkArray).collect::<Vec<_>>())
        .unwrap_or_default();
    let updatedEntities = json
        .get("update")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(parseUpdateArray)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mergedEntities = json
        .get("merge")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(parseMergeObject)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let userPreferences = json
        .get("user")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    Ok(ParsedAnalysis {
        mainProblem,
        extractedEntities,
        links,
        updatedEntities,
        mergedEntities,
        userPreferences,
    })
}

#[allow(non_snake_case)]
fn parseEntityArray(value: &Value) -> Option<ParsedEntity> {
    let array = value.as_array()?;
    Some(ParsedEntity {
        title: array.first()?.as_str()?.to_string(),
        content: array.get(1)?.as_str()?.to_string(),
        tags: stringArray(array.get(2)),
        folderPath: array
            .get(3)
            .and_then(Value::as_str)
            .map(ToString::to_string),
        aliasFor: array
            .get(4)
            .and_then(Value::as_str)
            .map(ToString::to_string),
    })
}

#[allow(non_snake_case)]
fn parseLinkArray(value: &Value) -> Option<ParsedLink> {
    let array = value.as_array()?;
    Some(ParsedLink {
        sourceTitle: array.first()?.as_str()?.to_string(),
        targetTitle: array.get(1)?.as_str()?.to_string(),
        type_: array.get(2)?.as_str()?.to_string(),
        description: array
            .get(3)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        weight: array.get(4).and_then(Value::as_f64).unwrap_or(1.0) as f32,
    })
}

#[allow(non_snake_case)]
fn parseUpdateArray(value: &Value) -> Option<ParsedUpdate> {
    let array = value.as_array()?;
    Some(ParsedUpdate {
        titleToUpdate: array.first()?.as_str()?.to_string(),
        newContent: array.get(1)?.as_str()?.to_string(),
        reason: array
            .get(2)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        newCredibility: array
            .get(3)
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        newImportance: array
            .get(4)
            .and_then(Value::as_f64)
            .map(|value| value as f32),
    })
}

#[allow(non_snake_case)]
fn parseMergeObject(value: &Value) -> Option<ParsedMerge> {
    let object = value.as_object()?;
    Some(ParsedMerge {
        sourceTitles: stringArray(object.get("source_titles")),
        newTitle: object.get("new_title")?.as_str()?.to_string(),
        newContent: object.get("new_content")?.as_str()?.to_string(),
        newTags: stringArray(object.get("new_tags")),
        folderPath: object
            .get("folder_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        reason: object
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

#[allow(non_snake_case)]
fn pruneToolResultContent(message: &str) -> String {
    let blocks = ChatMarkupRegex::tool_result_blocks(message);
    if blocks.is_empty() {
        return message.to_string();
    }
    let mut output = String::new();
    let mut cursor = 0;
    for block in blocks {
        output.push_str(&message[cursor..block.start]);
        let openEnd = block
            .raw
            .find('>')
            .map(|index| index + 1)
            .unwrap_or(block.raw.len());
        output.push_str(&block.raw[..openEnd]);
        output.push_str("[工具结果已省略]");
        output.push_str(&format!("</{}>", block.tag_name));
        cursor = block.end;
    }
    output.push_str(&message[cursor..]);
    output
}

#[allow(non_snake_case)]
fn removeMemoryTags(message: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    for (start, end) in tag_ranges(message, "memory") {
        output.push_str(&message[cursor..start]);
        cursor = end;
    }
    output.push_str(&message[cursor..]);
    output
}

#[allow(non_snake_case)]
fn newMemory(
    title: String,
    content: String,
    source: String,
    folderPath: Option<String>,
    credibility: f32,
    importance: f32,
) -> Memory {
    let now = Utc::now().timestamp_millis();
    Memory {
        id: 0,
        uuid: uuid::Uuid::new_v4().to_string(),
        title,
        content,
        contentType: "text".to_string(),
        source,
        credibility,
        importance,
        documentPath: None,
        isDocumentNode: false,
        chunkIndexFilePath: None,
        folderPath,
        createdAt: now,
        updatedAt: now,
        lastAccessedAt: now,
        tags: Vec::new(),
        properties: Vec::new(),
    }
}

#[allow(non_snake_case)]
fn buildTags(tags: Vec<String>) -> Vec<MemoryTag> {
    let mut result = Vec::new();
    for tag in tags {
        let name = tag.trim();
        if name.is_empty()
            || result
                .iter()
                .any(|existing: &MemoryTag| existing.name == name)
        {
            continue;
        }
        result.push(MemoryTag {
            id: result.len() as i64 + 1,
            name: name.to_string(),
        });
    }
    result
}

#[allow(non_snake_case)]
fn stringArray(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}
