use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::Utc;
use uuid::Uuid;

use crate::data::model::Memory::{
    Memory, MemoryGraph, MemoryGraphEdge, MemoryGraphNode, MemoryLink, MemoryTag,
};
use crate::data::model::MemoryExportModel::{
    ImportStrategy, MemoryExportData, MemoryImportResult, SerializableLink, SerializableMemory,
};
use crate::util::OperitPaths::{memoryLinkSqlitePath, memorySqlitePath};
use operit_store::ObjectBoxStore::ObjectBox;

#[derive(Clone)]
pub struct MemoryRepository {
    ownerKey: String,
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

    pub fn new(ownerKey: impl Into<String>) -> Self {
        let ownerKey = ownerKey.into();
        Self {
            ownerKey: ownerKey.clone(),
            memoryBox: ObjectBox::new(
                memorySqlitePath(&ownerKey).expect("memory sqlite path must be valid"),
                "Memory",
            ),
            linkBox: ObjectBox::new(
                memoryLinkSqlitePath(&ownerKey).expect("memory link sqlite path must be valid"),
                "MemoryLink",
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn ownerKey(&self) -> &str {
        &self.ownerKey
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

    #[allow(non_snake_case)]
    pub fn getMemoryGraph(&self) -> Result<MemoryGraph, String> {
        self.cleanupDanglingLinksIfNeeded()?;
        self.buildGraphFromMemories(self.memoryBox.all().map_err(|error| error.to_string())?)
    }

    #[allow(non_snake_case)]
    fn cleanupDanglingLinksIfNeeded(&self) -> Result<(), String> {
        let memories = self.memoryBox.all().map_err(|error| error.to_string())?;
        let memoryIds = memories
            .iter()
            .map(|memory| memory.id)
            .collect::<HashSet<_>>();
        let danglingLinkIds = self
            .linkBox
            .all()
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|link| {
                !memoryIds.contains(&link.sourceMemoryId)
                    || !memoryIds.contains(&link.targetMemoryId)
            })
            .map(|link| link.id)
            .collect::<Vec<_>>();
        if danglingLinkIds.is_empty() {
            return Ok(());
        }
        self.linkBox
            .removeByIds(&danglingLinkIds)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(non_snake_case)]
    fn buildGraphFromMemories(&self, memories: Vec<Memory>) -> Result<MemoryGraph, String> {
        let memoryByObjectId = memories
            .iter()
            .map(|memory| (memory.id, memory))
            .collect::<HashMap<_, _>>();
        let memoryUuids = memories
            .iter()
            .map(|memory| memory.uuid.clone())
            .collect::<HashSet<_>>();
        let nodes = memories
            .iter()
            .map(|memory| MemoryGraphNode {
                id: memory.uuid.clone(),
                label: memory.title.clone(),
                color: memoryGraphNodeColor(memory),
                metadata: HashMap::new(),
            })
            .collect::<Vec<_>>();
        let mut edgeKeys = HashSet::new();
        let mut edges = Vec::new();
        for link in self.linkBox.all().map_err(|error| error.to_string())? {
            let Some(sourceMemory) = memoryByObjectId.get(&link.sourceMemoryId) else {
                continue;
            };
            let Some(targetMemory) = memoryByObjectId.get(&link.targetMemoryId) else {
                continue;
            };
            if !memoryUuids.contains(&sourceMemory.uuid)
                || !memoryUuids.contains(&targetMemory.uuid)
            {
                continue;
            }
            let edgeKey = (
                link.id,
                sourceMemory.uuid.clone(),
                targetMemory.uuid.clone(),
                link.type_.clone(),
            );
            if !edgeKeys.insert(edgeKey) {
                continue;
            }
            edges.push(MemoryGraphEdge {
                id: link.id,
                sourceId: sourceMemory.uuid.clone(),
                targetId: targetMemory.uuid.clone(),
                label: Some(link.type_),
                weight: link.weight,
                metadata: HashMap::new(),
                isCrossFolderLink: Self::normalizeFolderPath(sourceMemory.folderPath.as_deref())
                    != Self::normalizeFolderPath(targetMemory.folderPath.as_deref()),
            });
        }
        Ok(MemoryGraph { nodes, edges })
    }

    #[allow(non_snake_case)]
    pub fn exportMemoriesToJson(&self) -> Result<String, String> {
        let memories = self
            .memoryBox
            .all()
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|memory| !memory.isDocumentNode)
            .collect::<Vec<_>>();
        let memoryUuids = memories
            .iter()
            .map(|memory| memory.uuid.clone())
            .collect::<HashSet<_>>();
        let memoryUuidById = memories
            .iter()
            .map(|memory| (memory.id, memory.uuid.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        let serializableMemories = memories
            .into_iter()
            .map(|memory| SerializableMemory {
                uuid: memory.uuid,
                title: memory.title,
                content: memory.content,
                contentType: memory.contentType,
                source: memory.source,
                credibility: memory.credibility,
                importance: memory.importance,
                folderPath: memory.folderPath,
                createdAt: memory.createdAt,
                updatedAt: memory.updatedAt,
                tagNames: memory.tags.into_iter().map(|tag| tag.name).collect(),
            })
            .collect::<Vec<_>>();
        let mut seenLinks = BTreeSet::new();
        let mut serializableLinks = Vec::new();
        for link in self.linkBox.all().map_err(|error| error.to_string())? {
            let Some(sourceUuid) = memoryUuidById.get(&link.sourceMemoryId) else {
                continue;
            };
            let Some(targetUuid) = memoryUuidById.get(&link.targetMemoryId) else {
                continue;
            };
            if !memoryUuids.contains(sourceUuid) || !memoryUuids.contains(targetUuid) {
                continue;
            }
            let key = (
                sourceUuid.clone(),
                targetUuid.clone(),
                link.type_.clone(),
                link.weight.to_bits(),
                link.description.clone(),
            );
            if !seenLinks.insert(key) {
                continue;
            }
            serializableLinks.push(SerializableLink {
                sourceUuid: sourceUuid.clone(),
                targetUuid: targetUuid.clone(),
                type_: link.type_,
                weight: link.weight,
                description: link.description,
            });
        }
        let exportData = MemoryExportData {
            memories: serializableMemories,
            links: serializableLinks,
            exportDate: nowMillis(),
            version: "1.0".to_string(),
        };
        serde_json::to_string_pretty(&exportData).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    pub fn importMemoriesFromJson(
        &self,
        jsonString: String,
        strategy: ImportStrategy,
    ) -> Result<MemoryImportResult, String> {
        let exportData: MemoryExportData =
            serde_json::from_str(&jsonString).map_err(|error| error.to_string())?;
        let mut result = MemoryImportResult::default();
        let mut uuidMap = std::collections::HashMap::<String, Memory>::new();

        for serializableMemory in exportData.memories {
            let existingMemory = self.findMemoryByUuid(&serializableMemory.uuid)?;
            match (existingMemory, &strategy) {
                (Some(existing), ImportStrategy::SKIP) => {
                    result.skippedMemories += 1;
                    uuidMap.insert(serializableMemory.uuid, existing);
                }
                (Some(mut existing), ImportStrategy::UPDATE) => {
                    existing.title = serializableMemory.title;
                    existing.content = serializableMemory.content;
                    existing.contentType = serializableMemory.contentType;
                    existing.source = serializableMemory.source;
                    existing.credibility = serializableMemory.credibility;
                    existing.importance = serializableMemory.importance;
                    existing.folderPath =
                        Self::normalizeFolderPath(serializableMemory.folderPath.as_deref());
                    existing.updatedAt = nowMillis();
                    existing.tags = buildTags(serializableMemory.tagNames);
                    let saved = self
                        .memoryBox
                        .put(existing)
                        .map_err(|error| error.to_string())?;
                    result.updatedMemories += 1;
                    uuidMap.insert(serializableMemory.uuid, saved);
                }
                (_, _) => {
                    let sourceUuid = serializableMemory.uuid.clone();
                    let forceNewUuid = strategy == ImportStrategy::CREATE_NEW;
                    let memory =
                        self.createMemoryFromSerializable(serializableMemory, forceNewUuid)?;
                    result.newMemories += 1;
                    uuidMap.insert(sourceUuid, memory);
                }
            }
        }

        for serializableLink in exportData.links {
            let Some(sourceMemory) = uuidMap.get(&serializableLink.sourceUuid) else {
                continue;
            };
            let Some(targetMemory) = uuidMap.get(&serializableLink.targetUuid) else {
                continue;
            };
            if self.memoryLinkExists(sourceMemory.id, targetMemory.id, &serializableLink.type_)? {
                continue;
            }
            let link = MemoryLink {
                id: 0,
                sourceMemoryId: sourceMemory.id,
                targetMemoryId: targetMemory.id,
                type_: serializableLink.type_,
                weight: serializableLink.weight,
                description: serializableLink.description,
            };
            self.linkBox.put(link).map_err(|error| error.to_string())?;
            result.newLinks += 1;
        }

        Ok(result)
    }

    #[allow(non_snake_case)]
    fn findMemoryByUuid(&self, uuid: &str) -> Result<Option<Memory>, String> {
        self.memoryBox
            .query()
            .filter({
                let uuid = uuid.to_string();
                move |memory| memory.uuid == uuid
            })
            .build()
            .findFirst()
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn createMemoryFromSerializable(
        &self,
        serializable: SerializableMemory,
        forceNewUuid: bool,
    ) -> Result<Memory, String> {
        let now = nowMillis();
        let memory = Memory {
            id: 0,
            uuid: if forceNewUuid {
                Uuid::new_v4().to_string()
            } else {
                serializable.uuid
            },
            title: serializable.title,
            content: serializable.content,
            contentType: serializable.contentType,
            source: serializable.source,
            credibility: serializable.credibility,
            importance: serializable.importance,
            documentPath: None,
            isDocumentNode: false,
            chunkIndexFilePath: None,
            folderPath: Self::normalizeFolderPath(serializable.folderPath.as_deref()),
            createdAt: serializable.createdAt,
            updatedAt: serializable.updatedAt,
            lastAccessedAt: now,
            tags: buildTags(serializable.tagNames),
            properties: Vec::new(),
        };
        self.memoryBox
            .put(memory)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn memoryLinkExists(
        &self,
        sourceMemoryId: i64,
        targetMemoryId: i64,
        linkType: &str,
    ) -> Result<bool, String> {
        Ok(self
            .linkBox
            .all()
            .map_err(|error| error.to_string())?
            .into_iter()
            .any(|link| {
                link.sourceMemoryId == sourceMemoryId
                    && link.targetMemoryId == targetMemoryId
                    && link.type_ == linkType
            }))
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

#[allow(non_snake_case)]
fn memoryGraphNodeColor(memory: &Memory) -> i64 {
    if memory.isDocumentNode {
        return 0xFF9575CD_i64;
    }
    match memory.tags.first().map(|tag| tag.name.as_str()) {
        Some("Person") => 0xFF81C784_i64,
        Some("Concept") => 0xFF64B5F6_i64,
        _ => 0xFFD3D3D3_i64,
    }
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
