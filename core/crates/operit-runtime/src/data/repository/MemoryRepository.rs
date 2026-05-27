use std::collections::{BTreeSet, HashSet};

use chrono::Utc;
use uuid::Uuid;

use crate::data::model::Memory::{Memory, MemoryLink, MemoryTag};
use operit_store::ObjectBoxStore::ObjectBox;
use operit_store::RuntimeStorePaths::default_data_dir;

#[derive(Clone)]
pub struct MemoryRepository {
    profileId: String,
    memoryBox: ObjectBox<Memory>,
    linkBox: ObjectBox<MemoryLink>,
}

#[derive(Clone, Debug)]
pub struct MemoryLinkInfo {
    pub link: MemoryLink,
    pub sourceTitle: String,
    pub targetTitle: String,
}

impl MemoryRepository {
    pub const STRONG_LINK: f32 = 1.0;
    pub const MEDIUM_LINK: f32 = 0.7;
    pub const WEAK_LINK: f32 = 0.3;

    pub fn new(profileId: impl Into<String>) -> Self {
        let profileId = profileId.into();
        let root = default_data_dir()
            .join("memory")
            .join(sanitizeProfileId(&profileId));
        Self {
            profileId,
            memoryBox: ObjectBox::new(root.join("Memory.sqlite"), "Memory"),
            linkBox: ObjectBox::new(root.join("MemoryLink.sqlite"), "MemoryLink"),
        }
    }

    pub fn profileId(&self) -> &str {
        &self.profileId
    }

    pub fn normalizeFolderPath(folderPath: Option<&str>) -> Option<String> {
        let raw = folderPath.map(str::trim)?;
        if raw.is_empty() {
            return None;
        }
        let parts = raw
            .replace('\\', "/")
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("/"))
        }
    }

    pub fn searchMemories(
        &self,
        query: &str,
        folderPath: Option<&str>,
        relevanceThreshold: f64,
        createdAtStartMs: Option<i64>,
        createdAtEndMs: Option<i64>,
    ) -> Result<Vec<Memory>, String> {
        let mut changed = false;
        let normalizedFolder = Self::normalizeFolderPath(folderPath);
        let query = query.trim();
        let wildcard = query == "*";
        let tokens = lexicalTokens(query);
        let mut scored = Vec::<(f64, Memory)>::new();

        self.memoryBox
            .editEntities(|memories| {
                for memory in memories.iter_mut() {
                    if normalizedFolder.as_deref() != memory.folderPath.as_deref()
                        && normalizedFolder.is_some()
                    {
                        continue;
                    }
                    if let Some(start) = createdAtStartMs {
                        if memory.createdAt < start {
                            continue;
                        }
                    }
                    if let Some(end) = createdAtEndMs {
                        if memory.createdAt > end {
                            continue;
                        }
                    }
                    let score = if wildcard {
                        1.0
                    } else {
                        lexicalScore(memory, &tokens)
                    };
                    if score >= relevanceThreshold {
                        memory.lastAccessedAt = nowMillis();
                        changed = true;
                        scored.push((score, memory.clone()));
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.1.updatedAt.cmp(&left.1.updatedAt))
        });
        if !changed {
            return Ok(Vec::new());
        }
        Ok(scored.into_iter().map(|(_, memory)| memory).collect())
    }

    pub fn findMemoryByTitle(&self, title: &str) -> Result<Option<Memory>, String> {
        let normalizedTitle = title.trim();
        self.memoryBox
            .query()
            .filter({
                let normalizedTitle = normalizedTitle.to_string();
                move |memory| memory.title == normalizedTitle
            })
            .build()
            .findFirst()
            .map_err(|error| error.to_string())
    }

    pub fn findMemoriesByTitle(&self, title: &str) -> Result<Vec<Memory>, String> {
        let normalizedTitle = title.trim();
        self.memoryBox
            .query()
            .filter({
                let normalizedTitle = normalizedTitle.to_string();
                move |memory| memory.title == normalizedTitle
            })
            .build()
            .find()
            .map_err(|error| error.to_string())
    }

    pub fn getMemoriesByFolderPath(&self, folderPath: &str) -> Result<Vec<Memory>, String> {
        let normalizedFolder = Self::normalizeFolderPath(Some(folderPath));
        self.memoryBox
            .query()
            .filter(move |memory| memory.folderPath == normalizedFolder)
            .build()
            .find()
            .map_err(|error| error.to_string())
    }

    pub fn createMemory(
        &self,
        title: String,
        content: String,
        contentType: String,
        source: String,
        folderPath: String,
        tags: Option<Vec<String>>,
    ) -> Result<Memory, String> {
        let now = nowMillis();
        let memory = Memory {
            id: 0,
            uuid: Uuid::new_v4().to_string(),
            title,
            content,
            contentType,
            source,
            credibility: 0.5,
            importance: 0.5,
            documentPath: None,
            isDocumentNode: false,
            chunkIndexFilePath: None,
            folderPath: Self::normalizeFolderPath(Some(&folderPath)),
            createdAt: now,
            updatedAt: now,
            lastAccessedAt: now,
            tags: buildTags(tags.unwrap_or_default()),
            properties: Vec::new(),
        };
        self.memoryBox
            .put(memory)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    pub fn saveMemory(&self, mut memory: Memory) -> Result<Memory, String> {
        let now = nowMillis();
        if memory.uuid.trim().is_empty() {
            memory.uuid = Uuid::new_v4().to_string();
        }
        if memory.createdAt <= 0 {
            memory.createdAt = now;
        }
        memory.updatedAt = now;
        if memory.lastAccessedAt <= 0 {
            memory.lastAccessedAt = now;
        }
        memory.folderPath = Self::normalizeFolderPath(memory.folderPath.as_deref());
        self.memoryBox
            .put(memory)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    pub fn addTagToMemory(&self, memoryId: i64, tagName: &str) -> Result<Option<Memory>, String> {
        let tagName = tagName.trim();
        if tagName.is_empty() {
            return Ok(self
                .memoryBox
                .get(memoryId)
                .map_err(|error| error.to_string())?);
        }
        self.memoryBox
            .editEntities(|memories| {
                let Some(memory) = memories.iter_mut().find(|memory| memory.id == memoryId) else {
                    return Ok(None);
                };
                if !memory.tags.iter().any(|tag| tag.name == tagName) {
                    let nextId = memory.tags.iter().map(|tag| tag.id).max().unwrap_or(0) + 1;
                    memory.tags.push(MemoryTag {
                        id: nextId,
                        name: tagName.to_string(),
                    });
                    memory.updatedAt = nowMillis();
                }
                Ok(Some(memory.clone()))
            })
            .map_err(|error| error.to_string())?
    }

    #[allow(non_snake_case)]
    pub fn mergeMemories(
        &self,
        sourceTitles: Vec<String>,
        newTitle: String,
        newContent: String,
        newTags: Vec<String>,
        folderPath: String,
    ) -> Result<Option<Memory>, String> {
        let mut sourceMemories = Vec::new();
        for title in &sourceTitles {
            sourceMemories.extend(self.findMemoriesByTitle(title)?);
        }
        if sourceMemories.is_empty() {
            return Ok(None);
        }
        let primaryId = sourceMemories[0].id;
        let sourceIds = sourceMemories
            .iter()
            .map(|memory| memory.id)
            .collect::<HashSet<_>>();
        let merged = self.updateMemory(
            primaryId,
            newTitle,
            newContent,
            "text".to_string(),
            "memory_analysis".to_string(),
            sourceMemories
                .iter()
                .map(|memory| memory.credibility)
                .fold(0.0_f32, f32::max),
            sourceMemories
                .iter()
                .map(|memory| memory.importance)
                .fold(0.0_f32, f32::max),
            Self::normalizeFolderPath(Some(&folderPath)),
            Some(newTags),
        )?;
        self.linkBox
            .editEntities(|links| {
                for link in links.iter_mut() {
                    if sourceIds.contains(&link.sourceMemoryId) {
                        link.sourceMemoryId = primaryId;
                    }
                    if sourceIds.contains(&link.targetMemoryId) {
                        link.targetMemoryId = primaryId;
                    }
                }
                links.retain(|link| link.sourceMemoryId != link.targetMemoryId);
            })
            .map_err(|error| error.to_string())?;
        let redundant = sourceIds
            .into_iter()
            .filter(|id| *id != primaryId)
            .collect::<Vec<_>>();
        self.memoryBox
            .removeByIds(&redundant)
            .map_err(|error| error.to_string())?;
        Ok(Some(merged))
    }

    pub fn updateMemory(
        &self,
        memoryId: i64,
        newTitle: String,
        newContent: String,
        newContentType: String,
        newSource: String,
        newCredibility: f32,
        newImportance: f32,
        newFolderPath: Option<String>,
        newTags: Option<Vec<String>>,
    ) -> Result<Memory, String> {
        self.memoryBox
            .editEntities(|memories| {
                let memory = memories
                    .iter_mut()
                    .find(|memory| memory.id == memoryId)
                    .ok_or_else(|| format!("Memory not found with id: {memoryId}"))?;
                memory.title = newTitle;
                memory.content = newContent;
                memory.contentType = newContentType;
                memory.source = newSource;
                memory.credibility = newCredibility.clamp(0.0, 1.0);
                memory.importance = newImportance.clamp(0.0, 1.0);
                memory.folderPath = Self::normalizeFolderPath(newFolderPath.as_deref());
                if let Some(tags) = newTags {
                    memory.tags = buildTags(tags);
                }
                memory.updatedAt = nowMillis();
                Ok(memory.clone())
            })
            .map_err(|error| error.to_string())?
    }

    pub fn deleteMemory(&self, memoryId: i64) -> Result<bool, String> {
        let deleted = self
            .memoryBox
            .remove(memoryId)
            .map_err(|error| error.to_string())?;
        if deleted {
            self.linkBox
                .editEntities(|links| {
                    links.retain(|link| {
                        link.sourceMemoryId != memoryId && link.targetMemoryId != memoryId
                    });
                })
                .map_err(|error| error.to_string())?;
        }
        Ok(deleted)
    }

    pub fn moveMemoriesToFolder(
        &self,
        memoryIds: &[i64],
        targetFolderPath: &str,
    ) -> Result<bool, String> {
        let selected = memoryIds.iter().copied().collect::<HashSet<_>>();
        let normalizedFolder = Self::normalizeFolderPath(Some(targetFolderPath));
        let mut changed = false;
        self.memoryBox
            .editEntities(|memories| {
                for memory in memories {
                    if selected.contains(&memory.id) {
                        memory.folderPath = normalizedFolder.clone();
                        memory.updatedAt = nowMillis();
                        changed = true;
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(changed)
    }

    pub fn linkMemories(
        &self,
        sourceMemoryId: i64,
        targetMemoryId: i64,
        type_: String,
        weight: f32,
        description: String,
    ) -> Result<MemoryLink, String> {
        let memories = self.memoryBox.all().map_err(|error| error.to_string())?;
        if !memories.iter().any(|memory| memory.id == sourceMemoryId) {
            return Err(format!("Source memory not found with id: {sourceMemoryId}"));
        }
        if !memories.iter().any(|memory| memory.id == targetMemoryId) {
            return Err(format!("Target memory not found with id: {targetMemoryId}"));
        }
        let link = MemoryLink {
            id: 0,
            sourceMemoryId,
            targetMemoryId,
            type_,
            weight: weight.clamp(0.0, 1.0),
            description,
        };
        self.linkBox.put(link).map_err(|error| error.to_string())
    }

    pub fn queryMemoryLinks(
        &self,
        linkId: Option<i64>,
        sourceMemoryId: Option<i64>,
        targetMemoryId: Option<i64>,
        linkType: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryLinkInfo>, String> {
        let links = self.linkBox.all().map_err(|error| error.to_string())?;
        let memories = self.memoryBox.all().map_err(|error| error.to_string())?;
        let mut result = Vec::new();
        for link in &links {
            if linkId.is_some_and(|value| value != link.id) {
                continue;
            }
            if sourceMemoryId.is_some_and(|value| value != link.sourceMemoryId) {
                continue;
            }
            if targetMemoryId.is_some_and(|value| value != link.targetMemoryId) {
                continue;
            }
            if linkType.is_some_and(|value| value != link.type_) {
                continue;
            }
            let sourceTitle = memories
                .iter()
                .find(|memory| memory.id == link.sourceMemoryId)
                .map(|memory| memory.title.clone())
                .ok_or_else(|| format!("Dangling source memory id: {}", link.sourceMemoryId))?;
            let targetTitle = memories
                .iter()
                .find(|memory| memory.id == link.targetMemoryId)
                .map(|memory| memory.title.clone())
                .ok_or_else(|| format!("Dangling target memory id: {}", link.targetMemoryId))?;
            result.push(MemoryLinkInfo {
                link: link.clone(),
                sourceTitle,
                targetTitle,
            });
            if result.len() >= limit {
                break;
            }
        }
        Ok(result)
    }

    pub fn findLinkById(&self, linkId: i64) -> Result<Option<MemoryLinkInfo>, String> {
        Ok(self
            .queryMemoryLinks(Some(linkId), None, None, None, 1)?
            .into_iter()
            .next())
    }

    pub fn updateLink(
        &self,
        linkId: i64,
        type_: String,
        weight: f32,
        description: String,
    ) -> Result<MemoryLinkInfo, String> {
        self.linkBox
            .editEntities(|links| {
                let link = links
                    .iter_mut()
                    .find(|link| link.id == linkId)
                    .ok_or_else(|| format!("Link not found with id: {linkId}"))?;
                link.type_ = type_;
                link.weight = weight.clamp(0.0, 1.0);
                link.description = description;
                Ok::<(), String>(())
            })
            .map_err(|error| error.to_string())??;
        self.findLinkById(linkId)?
            .ok_or_else(|| format!("Link not found with id: {linkId}"))
    }

    pub fn deleteLink(&self, linkId: i64) -> Result<bool, String> {
        self.linkBox
            .remove(linkId)
            .map_err(|error| error.to_string())
    }

    pub fn getAllFolderPaths(&self) -> Result<Vec<String>, String> {
        let mut folders = BTreeSet::new();
        for memory in self.memoryBox.all().map_err(|error| error.to_string())? {
            if let Some(folder) = memory.folderPath {
                folders.insert(folder);
            }
        }
        Ok(folders.into_iter().collect())
    }
}

fn sanitizeProfileId(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "default".to_string()
    } else {
        out
    }
}

fn nowMillis() -> i64 {
    Utc::now().timestamp_millis()
}

fn buildTags(tags: Vec<String>) -> Vec<MemoryTag> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for tag in tags {
        let name = tag.trim();
        if name.is_empty() || !seen.insert(name.to_string()) {
            continue;
        }
        result.push(MemoryTag {
            id: result.len() as i64 + 1,
            name: name.to_string(),
        });
    }
    result
}

fn lexicalTokens(query: &str) -> Vec<String> {
    query
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '|' | ',' | ';' | '，' | '；'))
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn lexicalScore(memory: &Memory, tokens: &[String]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }
    let haystack = format!(
        "{}\n{}\n{}\n{}",
        memory.title,
        memory.content,
        memory.source,
        memory
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    )
    .to_lowercase();
    let mut matched = 0usize;
    for token in tokens {
        if haystack.contains(token) {
            matched += 1;
        }
    }
    matched as f64 / tokens.len() as f64
}
