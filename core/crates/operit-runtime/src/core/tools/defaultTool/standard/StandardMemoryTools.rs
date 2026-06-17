use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone, Timelike};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutionManager, ToolExecutor, ToolValidationResult,
};
use crate::core::tools::ToolResultDataClasses::{
    stringResultData, LinkInfo, MemoryInfo, MemoryLinkQueryResultData, MemoryLinkResultData,
    MemoryQueryResultData, ToolResultData,
};
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::MemorySearchSettingsPreferences::MemorySearchSettingsPreferences;
use crate::data::repository::MemoryRepository::{MemoryLinkInfo, MemoryRepository};
use crate::data::repository::UserMarkdownRepository::UserMarkdownRepository;
use crate::util::OperitPaths::{
    characterMemoryOwnerKey, parseMemoryOwnerKey, sharedMemoryOwnerKey,
};

const MAX_QUERY_SNAPSHOTS_PER_OWNER: usize = 32;
const DEFAULT_RELEVANCE_THRESHOLD: f64 = 0.0;

#[derive(Clone, Debug)]
pub enum MemoryToolOperation {
    QueryMemory,
    GetMemoryByTitle,
    CreateMemory,
    UpdateMemory,
    DeleteMemory,
    MoveMemory,
    UpdateUserPreferences,
    LinkMemories,
    QueryMemoryLinks,
    UpdateMemoryLink,
    DeleteMemoryLink,
}

#[derive(Clone, Debug)]
pub struct MemoryToolExecutor {
    pub operation: MemoryToolOperation,
}

#[derive(Clone, Debug)]
struct QuerySnapshotState {
    seenMemoryKeys: HashSet<String>,
    lastAccessAtMs: i64,
}

#[derive(Clone, Debug)]
struct OwnedMemoryResult {
    ownerKey: String,
    memory: crate::data::model::Memory::Memory,
}

impl ToolExecutor for MemoryToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        if matches!(self.operation, MemoryToolOperation::QueryMemory)
            && optionalParameterValue(tool, "query")
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
        {
            return ToolValidationResult {
                valid: false,
                errorMessage: "Missing or empty required parameter: query".to_string(),
            };
        }
        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let result = match self.operation {
            MemoryToolOperation::QueryMemory => executeQueryMemory(tool),
            MemoryToolOperation::GetMemoryByTitle => executeGetMemoryByTitle(tool),
            MemoryToolOperation::CreateMemory => executeCreateMemory(tool),
            MemoryToolOperation::UpdateMemory => executeUpdateMemory(tool),
            MemoryToolOperation::DeleteMemory => executeDeleteMemory(tool),
            MemoryToolOperation::MoveMemory => executeMoveMemory(tool),
            MemoryToolOperation::UpdateUserPreferences => executeUpdateUserPreferences(tool),
            MemoryToolOperation::LinkMemories => executeLinkMemories(tool),
            MemoryToolOperation::QueryMemoryLinks => executeQueryMemoryLinks(tool),
            MemoryToolOperation::UpdateMemoryLink => executeUpdateMemoryLink(tool),
            MemoryToolOperation::DeleteMemoryLink => executeDeleteMemoryLink(tool),
        };
        vec![result]
    }
}

fn executeQueryMemory(tool: &AITool) -> ToolResult {
    let ownerKeys = match resolveReadableOwnerKeys(tool) {
        Ok(ownerKeys) => ownerKeys,
        Err(error) => return errorResult(tool, &error),
    };
    let query = parameterValue(tool, "query");
    if query.trim().is_empty() {
        return errorResult(tool, "Query parameter cannot be empty.");
    }
    let folderPath = optionalParameterValue(tool, "folder_path");
    let limitParam = optionalParameterValue(tool, "limit");
    let limit = match parseLimit(
        limitParam.as_deref(),
        if query.trim() == "*" { usize::MAX } else { 20 },
    ) {
        Ok(value) => value,
        Err(error) => return errorResult(tool, &error),
    };
    let threshold = match optionalParameterValue(tool, "threshold")
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.parse::<f64>())
    {
        Some(Ok(value)) if value >= 0.0 => value,
        Some(_) => return errorResult(tool, "Invalid threshold. Expected a non-negative number."),
        None => DEFAULT_RELEVANCE_THRESHOLD,
    };
    let startTime =
        match parseTimeBoundary(optionalParameterValue(tool, "start_time").as_deref(), false) {
            Ok(value) => value,
            Err(error) => return errorResult(tool, &error),
        };
    let endTime = match parseTimeBoundary(optionalParameterValue(tool, "end_time").as_deref(), true)
    {
        Ok(value) => value,
        Err(error) => return errorResult(tool, &error),
    };
    if let (Some(start), Some(end)) = (startTime, endTime) {
        if start > end {
            return errorResult(tool, "Invalid time range: start_time must be <= end_time.");
        }
    }

    let snapshotIdParam = optionalParameterValue(tool, "snapshot_id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let snapshotScope = ownerKeys.join("|");
    let (snapshotId, snapshotCreated) = resolveSnapshot(&snapshotScope, snapshotIdParam);
    let _settings = match MemorySearchSettingsPreferences::new(&snapshotScope).load() {
        Ok(settings) => settings,
        Err(error) => {
            return errorResult(
                tool,
                &format!("Failed to load memory search settings: {error}"),
            )
        }
    };
    let mut results = Vec::new();
    for ownerKey in &ownerKeys {
        match MemoryRepository::new(ownerKey.clone()).searchMemories(
            &query,
            folderPath.as_deref(),
            threshold,
            startTime,
            endTime,
        ) {
            Ok(ownerResults) => {
                results.extend(ownerResults.into_iter().map(|memory| OwnedMemoryResult {
                    ownerKey: ownerKey.clone(),
                    memory,
                }))
            }
            Err(error) => {
                return errorResult(tool, &format!("Failed to execute memory query: {error}"))
            }
        }
    }

    let (excluded, returned) = selectSnapshotResults(&snapshotScope, &snapshotId, results, limit);
    successData(
        tool,
        ToolResultData::MemoryQueryResultData(memoryQueryResultData(
            &returned,
            Some(snapshotId),
            snapshotCreated,
            excluded,
        )),
    )
}

fn executeGetMemoryByTitle(tool: &AITool) -> ToolResult {
    let title = parameterValue(tool, "title");
    if title.trim().is_empty() {
        return errorResult(tool, "title parameter is required");
    }
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey.clone());
    match repository.findMemoryByTitle(&title) {
        Ok(Some(memory)) => successData(
            tool,
            ToolResultData::MemoryQueryResultData(memoryQueryResultData(
                &[OwnedMemoryResult { ownerKey, memory }],
                None,
                false,
                0,
            )),
        ),
        Ok(None) => errorResult(tool, &format!("Memory not found with title: {title}")),
        Err(error) => errorResult(tool, &format!("Failed to get memory by title: {error}")),
    }
}

fn executeCreateMemory(tool: &AITool) -> ToolResult {
    let title = parameterValue(tool, "title");
    let content = parameterValue(tool, "content");
    if title.trim().is_empty() || content.trim().is_empty() {
        return errorResult(tool, "Both title and content parameters are required");
    }
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let contentType =
        optionalParameterValue(tool, "content_type").unwrap_or_else(|| "text/plain".to_string());
    let source = optionalParameterValue(tool, "source").unwrap_or_else(|| "ai_created".to_string());
    let folderPath = optionalParameterValue(tool, "folder_path").unwrap_or_default();
    let tags = optionalParameterValue(tool, "tags").map(parseTags);
    match repository.createMemory(
        title.clone(),
        content,
        contentType,
        source,
        folderPath,
        tags,
    ) {
        Ok(memory) => success(
            tool,
            format!(
                "Successfully created memory: '{}' (UUID: {})",
                title, memory.uuid
            ),
        ),
        Err(error) => errorResult(tool, &format!("Failed to create memory: {error}")),
    }
}

fn executeUpdateMemory(tool: &AITool) -> ToolResult {
    let oldTitle = parameterValue(tool, "old_title");
    if oldTitle.trim().is_empty() {
        return errorResult(
            tool,
            "old_title parameter is required to identify the memory",
        );
    }
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let memory = match repository.findMemoryByTitle(&oldTitle) {
        Ok(Some(memory)) => memory,
        Ok(None) => return errorResult(tool, &format!("Memory not found with title: {oldTitle}")),
        Err(error) => return errorResult(tool, &format!("Failed to update memory: {error}")),
    };
    let newTitle =
        optionalParameterValue(tool, "new_title").unwrap_or_else(|| memory.title.clone());
    let newContent =
        optionalParameterValue(tool, "content").unwrap_or_else(|| memory.content.clone());
    let newContentType =
        optionalParameterValue(tool, "content_type").unwrap_or_else(|| memory.contentType.clone());
    let newSource = optionalParameterValue(tool, "source").unwrap_or_else(|| memory.source.clone());
    let newCredibility = optionalParameterValue(tool, "credibility")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(memory.credibility);
    let newImportance = optionalParameterValue(tool, "importance")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(memory.importance);
    let newFolderPath = optionalParameterValue(tool, "folder_path").or(memory.folderPath.clone());
    let newTags = optionalParameterValue(tool, "tags").map(parseTags);
    match repository.updateMemory(
        memory.id,
        newTitle.clone(),
        newContent,
        newContentType,
        newSource,
        newCredibility,
        newImportance,
        newFolderPath,
        newTags,
    ) {
        Ok(_) => success(
            tool,
            format!("Successfully updated memory from '{oldTitle}' to '{newTitle}'"),
        ),
        Err(error) => errorResult(tool, &format!("Failed to update memory: {error}")),
    }
}

fn executeDeleteMemory(tool: &AITool) -> ToolResult {
    let title = parameterValue(tool, "title");
    if title.trim().is_empty() {
        return errorResult(tool, "title parameter is required to identify the memory");
    }
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let memory = match repository.findMemoryByTitle(&title) {
        Ok(Some(memory)) => memory,
        Ok(None) => return errorResult(tool, &format!("Memory not found with title: {title}")),
        Err(error) => return errorResult(tool, &format!("Failed to delete memory: {error}")),
    };
    match repository.deleteMemory(memory.id) {
        Ok(true) => success(tool, format!("Successfully deleted memory: '{title}'")),
        Ok(false) => errorResult(tool, "Failed to delete memory"),
        Err(error) => errorResult(tool, &format!("Failed to delete memory: {error}")),
    }
}

fn executeMoveMemory(tool: &AITool) -> ToolResult {
    let targetFolderPath = match optionalParameterValue(tool, "target_folder_path") {
        Some(value) => value,
        None => return errorResult(tool, "target_folder_path parameter is required"),
    };
    let sourceFolderPath = optionalParameterValue(tool, "source_folder_path");
    let hasSourceFolderParam = tool
        .parameters
        .iter()
        .any(|parameter| parameter.name == "source_folder_path");
    let titles = optionalParameterValue(tool, "titles")
        .map(parseTitles)
        .unwrap_or_default();
    if titles.is_empty() && !hasSourceFolderParam {
        return errorResult(
            tool,
            "Provide titles and/or source_folder_path to select memories to move",
        );
    }

    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let mut selected = Vec::new();
    if !titles.is_empty() {
        for title in titles {
            match repository.findMemoriesByTitle(&title) {
                Ok(memories) => selected.extend(memories),
                Err(error) => {
                    return errorResult(tool, &format!("Failed to move memories: {error}"))
                }
            }
        }
    }
    if hasSourceFolderParam {
        match repository.getMemoriesByFolderPath(sourceFolderPath.as_deref().unwrap_or("")) {
            Ok(memories) => {
                if selected.is_empty() {
                    selected = memories;
                } else {
                    let folderIds = memories
                        .into_iter()
                        .map(|memory| memory.id)
                        .collect::<HashSet<_>>();
                    selected.retain(|memory| folderIds.contains(&memory.id));
                }
            }
            Err(error) => return errorResult(tool, &format!("Failed to move memories: {error}")),
        }
    }
    let mut ids = selected
        .into_iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return errorResult(tool, "No matching memories found to move");
    }
    match repository.moveMemoriesToFolder(&ids, &targetFolderPath) {
        Ok(true) => success(
            tool,
            format!(
                "Successfully moved {} memories to '{}'",
                ids.len(),
                targetFolderPath
            ),
        ),
        Ok(false) => errorResult(tool, "Failed to move selected memories"),
        Err(error) => errorResult(tool, &format!("Failed to move memories: {error}")),
    }
}

fn executeUpdateUserPreferences(tool: &AITool) -> ToolResult {
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let content = parameterValue(tool, "content");
    if content.trim().is_empty() {
        return errorResult(tool, "content parameter is required");
    }
    match UserMarkdownRepository::new(ownerKey).writeUserMarkdown(content) {
        Ok(_) => success(tool, "Successfully updated USER.md".to_string()),
        Err(error) => errorResult(tool, &format!("Failed to update USER.md: {error}")),
    }
}

fn executeLinkMemories(tool: &AITool) -> ToolResult {
    let sourceTitle = parameterValue(tool, "source_title");
    let targetTitle = parameterValue(tool, "target_title");
    if sourceTitle.trim().is_empty() || targetTitle.trim().is_empty() {
        return errorResult(
            tool,
            "Both source_title and target_title parameters are required",
        );
    }
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let source = match repository.findMemoryByTitle(&sourceTitle) {
        Ok(Some(memory)) => memory,
        Ok(None) => {
            return errorResult(
                tool,
                &format!("Source memory not found with title: {sourceTitle}"),
            )
        }
        Err(error) => return errorResult(tool, &format!("Failed to link memories: {error}")),
    };
    let target = match repository.findMemoryByTitle(&targetTitle) {
        Ok(Some(memory)) => memory,
        Ok(None) => {
            return errorResult(
                tool,
                &format!("Target memory not found with title: {targetTitle}"),
            )
        }
        Err(error) => return errorResult(tool, &format!("Failed to link memories: {error}")),
    };
    let linkType =
        optionalParameterValue(tool, "link_type").unwrap_or_else(|| "related".to_string());
    let weight = optionalParameterValue(tool, "weight")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(MemoryRepository::MEDIUM_LINK)
        .clamp(0.0, 1.0);
    let description = optionalParameterValue(tool, "description").unwrap_or_default();
    match repository.linkMemories(
        source.id,
        target.id,
        linkType.clone(),
        weight,
        description.clone(),
    ) {
        Ok(_) => successData(
            tool,
            ToolResultData::MemoryLinkResultData(MemoryLinkResultData {
                sourceTitle,
                targetTitle,
                linkType,
                weight,
                description,
            }),
        ),
        Err(error) => errorResult(tool, &format!("Failed to link memories: {error}")),
    }
}

fn executeQueryMemoryLinks(tool: &AITool) -> ToolResult {
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let linkId = match optionalParameterValue(tool, "link_id")
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.parse::<i64>())
    {
        Some(Ok(value)) => Some(value),
        Some(Err(_)) => return errorResult(tool, "Invalid link_id. Expected integer."),
        None => None,
    };
    let sourceMemoryId = match optionalParameterValue(tool, "source_title")
        .filter(|value| !value.trim().is_empty())
    {
        Some(title) => match repository.findMemoryByTitle(&title) {
            Ok(Some(memory)) => Some(memory.id),
            Ok(None) => {
                return errorResult(
                    tool,
                    &format!("Source memory not found with title: {title}"),
                )
            }
            Err(error) => {
                return errorResult(tool, &format!("Failed to query memory links: {error}"))
            }
        },
        None => None,
    };
    let targetMemoryId = match optionalParameterValue(tool, "target_title")
        .filter(|value| !value.trim().is_empty())
    {
        Some(title) => match repository.findMemoryByTitle(&title) {
            Ok(Some(memory)) => Some(memory.id),
            Ok(None) => {
                return errorResult(
                    tool,
                    &format!("Target memory not found with title: {title}"),
                )
            }
            Err(error) => {
                return errorResult(tool, &format!("Failed to query memory links: {error}"))
            }
        },
        None => None,
    };
    let limit = match parseLimit(optionalParameterValue(tool, "limit").as_deref(), 20) {
        Ok(value) => value.min(200),
        Err(error) => return errorResult(tool, &error),
    };
    let linkType = optionalParameterValue(tool, "link_type");
    match repository.queryMemoryLinks(
        linkId,
        sourceMemoryId,
        targetMemoryId,
        linkType.as_deref(),
        limit,
    ) {
        Ok(links) => successData(
            tool,
            ToolResultData::MemoryLinkQueryResultData(memoryLinkQueryResultData(&links)),
        ),
        Err(error) => errorResult(tool, &format!("Failed to query memory links: {error}")),
    }
}

fn executeUpdateMemoryLink(tool: &AITool) -> ToolResult {
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let newLinkType = optionalParameterValue(tool, "new_link_type");
    let newWeight =
        optionalParameterValue(tool, "weight").and_then(|value| value.parse::<f32>().ok());
    let newDescription = optionalParameterValue(tool, "description");
    if newLinkType
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
        && newWeight.is_none()
        && newDescription.is_none()
    {
        return errorResult(
            tool,
            "At least one of new_link_type, weight, description must be provided",
        );
    }
    let link = match resolveLink(tool, &repository) {
        Ok(link) => link,
        Err(error) => return errorResult(tool, &error),
    };
    let type_ = newLinkType.unwrap_or_else(|| link.link.type_.clone());
    let weight = newWeight.unwrap_or(link.link.weight).clamp(0.0, 1.0);
    let description = newDescription.unwrap_or_else(|| link.link.description.clone());
    match repository.updateLink(link.link.id, type_, weight, description) {
        Ok(updated) => successData(
            tool,
            ToolResultData::MemoryLinkQueryResultData(memoryLinkQueryResultData(&[updated])),
        ),
        Err(error) => errorResult(tool, &format!("Failed to update memory link: {error}")),
    }
}

fn executeDeleteMemoryLink(tool: &AITool) -> ToolResult {
    let ownerKey = match resolveTargetOwnerKey(tool) {
        Ok(ownerKey) => ownerKey,
        Err(error) => return errorResult(tool, &error),
    };
    let repository = MemoryRepository::new(ownerKey);
    let link = match resolveLink(tool, &repository) {
        Ok(link) => link,
        Err(error) => return errorResult(tool, &error),
    };
    match repository.deleteLink(link.link.id) {
        Ok(true) => success(
            tool,
            format!("Successfully deleted memory link: {}", link.link.id),
        ),
        Ok(false) => errorResult(
            tool,
            &format!("Failed to delete memory link with id: {}", link.link.id),
        ),
        Err(error) => errorResult(tool, &format!("Failed to delete memory link: {error}")),
    }
}

fn resolveLink(tool: &AITool, repository: &MemoryRepository) -> Result<MemoryLinkInfo, String> {
    let linkId =
        optionalParameterValue(tool, "link_id").and_then(|value| value.parse::<i64>().ok());
    if let Some(linkId) = linkId {
        return repository
            .findLinkById(linkId)?
            .ok_or_else(|| format!("Link not found with id: {linkId}"));
    }
    let sourceTitle = parameterValue(tool, "source_title");
    let targetTitle = parameterValue(tool, "target_title");
    if sourceTitle.trim().is_empty() || targetTitle.trim().is_empty() {
        return Err("Provide link_id, or provide both source_title and target_title".to_string());
    }
    let source = repository
        .findMemoryByTitle(&sourceTitle)?
        .ok_or_else(|| format!("Source memory not found with title: {sourceTitle}"))?;
    let target = repository
        .findMemoryByTitle(&targetTitle)?
        .ok_or_else(|| format!("Target memory not found with title: {targetTitle}"))?;
    let linkType = optionalParameterValue(tool, "link_type");
    let links = repository.queryMemoryLinks(
        None,
        Some(source.id),
        Some(target.id),
        linkType.as_deref(),
        2,
    )?;
    match links.len() {
        0 => Err("No matching link found".to_string()),
        1 => Ok(links[0].clone()),
        _ => {
            Err("Multiple links matched. Provide link_id or a more specific link_type.".to_string())
        }
    }
}

fn resolveCallerCardId(tool: &AITool) -> Option<String> {
    let explicitCallerCardId = tool
        .parameters
        .iter()
        .find(|parameter| parameter.name == "caller_card_id")
        .map(|parameter| parameter.value.trim().to_string())
        .filter(|value| !value.is_empty());
    if explicitCallerCardId.is_some() {
        return explicitCallerCardId;
    }
    ToolExecutionManager::currentToolRuntimeContext()
        .and_then(|context| context.callerCardId)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn explicitTargetOwnerKey(tool: &AITool) -> Option<String> {
    optionalParameterValue(tool, "target_owner_key")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn resolveTargetOwnerKey(tool: &AITool) -> Result<String, String> {
    let ownerKey = explicitTargetOwnerKey(tool)
        .ok_or_else(|| "target_owner_key parameter is required".to_string())?;
    parseMemoryOwnerKey(&ownerKey)?;
    Ok(ownerKey)
}

fn resolveReadableOwnerKeys(tool: &AITool) -> Result<Vec<String>, String> {
    if let Some(ownerKey) = explicitTargetOwnerKey(tool) {
        parseMemoryOwnerKey(&ownerKey)?;
        return Ok(vec![ownerKey]);
    }
    let callerCardId = resolveCallerCardId(tool)
        .ok_or_else(|| "caller_card_id or target_owner_key parameter is required".to_string())?;
    let characterCard = CharacterCardManager::getInstance()
        .getCharacterCard(&callerCardId)
        .map_err(|error| error.to_string())?;
    let mut ownerKeys = vec![characterMemoryOwnerKey(&characterCard.id)?];
    for mount in characterCard.sharedMemoryMounts {
        if mount.readable {
            ownerKeys.push(sharedMemoryOwnerKey(&mount.sharedMemoryId)?);
        }
    }
    Ok(ownerKeys)
}

fn parseTimeBoundary(value: Option<&str>, isEnd: bool) -> Result<Option<i64>, String> {
    let Some(trimmed) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Ok(parsed) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M") {
        let parsed = parsed
            .with_second(if isEnd { 59 } else { 0 })
            .and_then(|value| value.with_nanosecond(if isEnd { 999_000_000 } else { 0 }))
            .ok_or_else(|| "Invalid date-time value.".to_string())?;
        return localTimestampMillis(parsed);
    }
    if let Ok(parsed) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let parsed = parsed
            .and_hms_milli_opt(
                if isEnd { 23 } else { 0 },
                if isEnd { 59 } else { 0 },
                if isEnd { 59 } else { 0 },
                if isEnd { 999 } else { 0 },
            )
            .ok_or_else(|| "Invalid date value.".to_string())?;
        return localTimestampMillis(parsed);
    }
    if isEnd {
        Err("Invalid end_time. Expected format YYYY-MM-DD or YYYY-MM-DD HH:mm.".to_string())
    } else {
        Err("Invalid start_time. Expected format YYYY-MM-DD or YYYY-MM-DD HH:mm.".to_string())
    }
}

fn localTimestampMillis(value: NaiveDateTime) -> Result<Option<i64>, String> {
    let Some(local) = Local.from_local_datetime(&value).single() else {
        return Err("Local time is ambiguous or invalid.".to_string());
    };
    Ok(Some(local.timestamp_millis()))
}

fn resolveSnapshot(ownerScope: &str, requestedSnapshotId: Option<String>) -> (String, bool) {
    let mut snapshots = snapshotStore()
        .lock()
        .expect("memory query snapshot store mutex poisoned");
    let ownerSnapshots = snapshots.entry(ownerScope.to_string()).or_default();
    trimOldSnapshots(ownerSnapshots);
    let id = requestedSnapshotId.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let created = !ownerSnapshots.contains_key(&id);
    ownerSnapshots
        .entry(id.clone())
        .or_insert(QuerySnapshotState {
            seenMemoryKeys: HashSet::new(),
            lastAccessAtMs: operit_host_api::TimeUtils::currentTimeMillis(),
        });
    (id, created)
}

fn selectSnapshotResults(
    ownerScope: &str,
    snapshotId: &str,
    results: Vec<OwnedMemoryResult>,
    limit: usize,
) -> (usize, Vec<OwnedMemoryResult>) {
    let mut snapshots = snapshotStore()
        .lock()
        .expect("memory query snapshot store mutex poisoned");
    let snapshot = snapshots
        .entry(ownerScope.to_string())
        .or_default()
        .entry(snapshotId.to_string())
        .or_insert(QuerySnapshotState {
            seenMemoryKeys: HashSet::new(),
            lastAccessAtMs: operit_host_api::TimeUtils::currentTimeMillis(),
        });
    let excluded = results
        .iter()
        .filter(|entry| snapshot.seenMemoryKeys.contains(&memorySnapshotKey(entry)))
        .count();
    let selected = results
        .into_iter()
        .filter(|entry| !snapshot.seenMemoryKeys.contains(&memorySnapshotKey(entry)))
        .take(limit)
        .collect::<Vec<_>>();
    for entry in &selected {
        snapshot.seenMemoryKeys.insert(memorySnapshotKey(entry));
    }
    snapshot.lastAccessAtMs = operit_host_api::TimeUtils::currentTimeMillis();
    (excluded, selected)
}

fn trimOldSnapshots(ownerSnapshots: &mut HashMap<String, QuerySnapshotState>) {
    if ownerSnapshots.len() <= MAX_QUERY_SNAPSHOTS_PER_OWNER {
        return;
    }
    let overflow = ownerSnapshots.len() - MAX_QUERY_SNAPSHOTS_PER_OWNER;
    let mut entries = ownerSnapshots
        .iter()
        .map(|(id, state)| (id.clone(), state.lastAccessAtMs))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(_, lastAccessAtMs)| *lastAccessAtMs);
    for (id, _) in entries.into_iter().take(overflow) {
        ownerSnapshots.remove(&id);
    }
}

fn memorySnapshotKey(entry: &OwnedMemoryResult) -> String {
    format!("{}:{}", entry.ownerKey, entry.memory.id)
}

fn snapshotStore() -> &'static Mutex<HashMap<String, HashMap<String, QuerySnapshotState>>> {
    static SNAPSHOTS: OnceLock<Mutex<HashMap<String, HashMap<String, QuerySnapshotState>>>> =
        OnceLock::new();
    SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn memoryQueryResultData(
    memories: &[OwnedMemoryResult],
    snapshotId: Option<String>,
    snapshotCreated: bool,
    excludedBySnapshotCount: usize,
) -> MemoryQueryResultData {
    MemoryQueryResultData {
        memories: memories.iter().map(memoryInfo).collect(),
        snapshotId,
        snapshotCreated,
        excludedBySnapshotCount: excludedBySnapshotCount as i32,
    }
}

fn memoryInfo(entry: &OwnedMemoryResult) -> MemoryInfo {
    let memory = &entry.memory;
    MemoryInfo {
        ownerKey: entry.ownerKey.clone(),
        title: memory.title.clone(),
        content: memory.content.clone(),
        source: memory.source.clone(),
        tags: memory.tags.iter().map(|tag| tag.name.clone()).collect(),
        createdAt: formatMillis(memory.createdAt),
        chunkInfo: None,
        chunkIndices: None,
    }
}

fn memoryLinkQueryResultData(links: &[MemoryLinkInfo]) -> MemoryLinkQueryResultData {
    MemoryLinkQueryResultData {
        totalCount: links.len() as i32,
        links: links
            .iter()
            .map(|info| LinkInfo {
                linkId: info.link.id,
                sourceTitle: info.sourceTitle.clone(),
                targetTitle: info.targetTitle.clone(),
                linkType: info.link.type_.clone(),
                weight: info.link.weight,
                description: info.link.description.clone(),
            })
            .collect(),
    }
}

fn formatMemoryResults(
    query: &str,
    memories: &[crate::data::model::Memory::Memory],
    snapshotId: Option<&str>,
    snapshotCreated: bool,
    excludedBySnapshotCount: usize,
    settingsText: &str,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("query: {query}"));
    if let Some(snapshotId) = snapshotId {
        lines.push(format!("snapshot_id: {snapshotId}"));
        lines.push(format!("snapshot_created: {snapshotCreated}"));
        lines.push(format!(
            "excluded_by_snapshot_count: {excludedBySnapshotCount}"
        ));
    }
    if !settingsText.is_empty() {
        lines.push(settingsText.to_string());
    }
    lines.push(format!("total_count: {}", memories.len()));
    for memory in memories {
        let folder = memory.folderPath.clone().unwrap_or_default();
        let tags = memory
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "\n- title: {}\n  content: {}\n  source: {}\n  tags: {}\n  folder_path: {}\n  created_at: {}",
            memory.title,
            wildcardSummary(memory.content.as_str(), query),
            memory.source,
            tags,
            folder,
            formatMillis(memory.createdAt)
        ));
    }
    lines.join("\n")
}

fn wildcardSummary(content: &str, query: &str) -> String {
    if query.trim() == "*" && content.chars().count() > 10 {
        format!("{}...", content.chars().take(10).collect::<String>())
    } else {
        content.to_string()
    }
}

fn formatMemoryLinks(links: &[MemoryLinkInfo]) -> String {
    let mut lines = Vec::new();
    lines.push(format!("total_count: {}", links.len()));
    for info in links {
        lines.push(format!(
            "\n- link_id: {}\n  source_title: {}\n  target_title: {}\n  link_type: {}\n  weight: {}\n  description: {}",
            info.link.id,
            info.sourceTitle,
            info.targetTitle,
            info.link.type_,
            info.link.weight,
            info.link.description
        ));
    }
    lines.join("\n")
}

fn formatMillis(value: i64) -> String {
    let Some(value) = Local.timestamp_millis_opt(value).single() else {
        return value.to_string();
    };
    value.format("%Y-%m-%d %H:%M").to_string()
}

fn parseLimit(value: Option<&str>, defaultLimit: usize) -> Result<usize, String> {
    let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(defaultLimit.max(1));
    };
    let parsed = raw
        .parse::<usize>()
        .map_err(|_| "Invalid limit. Expected integer.".to_string())?;
    Ok(parsed.max(1))
}

fn parseTags(raw: String) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parseTitles(raw: String) -> Vec<String> {
    raw.split(|ch| matches!(ch, ',' | '\n' | '|'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn success(tool: &AITool, result: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: stringResultData(result),
        error: None,
    }
}

fn successData(tool: &AITool, data: ToolResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: data,
        error: None,
    }
}

fn errorResult(tool: &AITool, message: &str) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(message.to_string()),
    }
}

fn parameterValue(tool: &AITool, name: &str) -> String {
    optionalParameterValue(tool, name).unwrap_or_default()
}

fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.clone())
}
