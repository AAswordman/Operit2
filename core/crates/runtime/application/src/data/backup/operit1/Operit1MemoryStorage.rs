/// Builds the exact resource-copy plan before opening the ZIP for streaming extraction.
fn buildSnapshotFileCopyPlan(
    parsed: &ParsedOperit1Snapshot,
) -> Result<SnapshotFileCopyPlan, String> {
    let mut plan = SnapshotFileCopyPlan {
        items: Vec::new(),
        workspaceIds: BTreeSet::new(),
        importedFiles: 0,
        importedExternalFiles: 0,
        importedWorkspaceFiles: 0,
    };
    for entry in parsed.archive.entries.keys() {
        let Some(relative) = entry.strip_prefix(ENTRY_WORKSPACE_FILES_PREFIX) else {
            continue;
        };
        validateRelativePath(relative)?;
        let (workspaceId, rest) = splitWorkspaceRelativePath(relative)?;
        plan.workspaceIds.insert(workspaceId.to_string());
        if !rest.is_empty() {
            plan.items.push(SnapshotFileCopyItem {
                sourceEntry: entry.clone(),
                targetPath: format!("{WORKSPACE_DIR_PATH}/{workspaceId}/{rest}"),
            });
            plan.importedWorkspaceFiles += 1;
        }
    }
    for entry in parsed.archive.entries.keys() {
        if !entryMatchesCopyPrefix(entry, ENTRY_FILES_PREFIX) {
            continue;
        }
        let relative = entry
            .strip_prefix(ENTRY_FILES_PREFIX)
            .ok_or_else(|| format!("快照资源路径前缀不匹配：{entry}"))?;
        validateRelativePath(relative)?;
        plan.items.push(SnapshotFileCopyItem {
            sourceEntry: entry.clone(),
            targetPath: format!(
                "{}/{relative}",
                RUNTIME_IMPORTED_OPERIT1_FILES_DIR_PATH.trim_end_matches('/')
            ),
        });
        plan.importedFiles += 1;
    }
    for entry in parsed.archive.entries.keys() {
        if !entryMatchesCopyPrefix(entry, ENTRY_EXTERNAL_FILES_PREFIX) {
            continue;
        }
        let relative = entry
            .strip_prefix(ENTRY_EXTERNAL_FILES_PREFIX)
            .ok_or_else(|| format!("快照资源路径前缀不匹配：{entry}"))?;
        validateRelativePath(relative)?;
        plan.items.push(SnapshotFileCopyItem {
            sourceEntry: entry.clone(),
            targetPath: format!(
                "{}/{relative}",
                RUNTIME_IMPORTED_OPERIT1_EXTERNAL_FILES_DIR_PATH.trim_end_matches('/')
            ),
        });
        plan.importedExternalFiles += 1;
    }
    Ok(plan)
}

/// Recognizes declared Operit1 memory directories without treating user workspace documents as memory storage.
#[allow(non_snake_case)]
fn isOperit1LegacyMemoryEntry(entry: &str) -> bool {
    let Some(relative) = entry.strip_prefix(ENTRY_FILES_PREFIX) else {
        return false;
    };
    let Some((directory, file)) = relative.split_once('/') else {
        return false;
    };
    (directory == "objectbox" || directory.starts_with("objectbox_")) && matches!(file, "data.mdb" | "lock.mdb")
        || directory == "memory-space-profiles"
}

/// Returns whether an entry is an adopted non-workspace resource, leaving legacy preferences and memory source data unmodified.
fn entryMatchesCopyPrefix(entry: &str, sourcePrefix: &str) -> bool {
    entry.starts_with(sourcePrefix)
        && !entry.starts_with(ENTRY_DATASTORE_PREFIX)
        && !isOperit1LegacyMemoryEntry(entry)
        && !(sourcePrefix == ENTRY_FILES_PREFIX && entry.starts_with(ENTRY_WORKSPACE_FILES_PREFIX))
}

/// Reads the declared source layout for splitWorkspaceRelativePath.
#[allow(non_snake_case)]
fn splitWorkspaceRelativePath(relative: &str) -> Result<(&str, &str), String> {
    if let Some((workspaceId, rest)) = relative.split_once('/') {
        validateWorkspaceIdSegment(workspaceId)?;
        validateRelativePath(rest)?;
        return Ok((workspaceId, rest));
    }
    validateWorkspaceIdSegment(relative)?;
    Ok((relative, ""))
}

/// Reads the declared source layout for validateWorkspaceIdSegment.
#[allow(non_snake_case)]
fn validateWorkspaceIdSegment(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
    {
        return Err(format!("Operit1 工作区 ID 无效：{value}"));
    }
    Ok(())
}

/// Reads the declared source layout for workspaceVfsPath.
#[allow(non_snake_case)]
fn workspaceVfsPath(workspaceId: &str, rest: &str) -> String {
    if rest.trim().is_empty() {
        format!("/app/workspaces/{workspaceId}")
    } else {
        format!("/app/workspaces/{workspaceId}/{rest}")
    }
}

/// Joins a validated relative path to one stable virtual file-system root.
fn joinVirtualPath(root: &str, relative: &str) -> String {
    format!("{}/{relative}", root.trim_end_matches('/'))
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1MemoryRecord {
    id: i64,
    uuid: String,
    title: String,
    content: String,
    contentType: String,
    source: String,
    credibility: f32,
    importance: f32,
    folderPath: Option<String>,
    createdAt: i64,
    updatedAt: i64,
    isDocumentNode: bool,
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1MemoryLinkRecord {
    id: i64,
    type_: String,
    weight: f32,
    description: String,
    sourceId: i64,
    targetId: i64,
}

#[derive(Clone, Debug)]
struct Operit1MemoryTagRecord {
    id: i64,
    name: String,
}

/// Reads full Operit1 entities into plugin records instead of passing through the lossy portable memory export.
#[allow(non_snake_case)]
fn buildOperit1PluginMemoryRecords(
    storage: &dyn RuntimeStorageHost,
    path: &str,
    length: u64,
    owner: &str,
    plan: &SnapshotFileImportPlan,
    archive: &OperitChatArchive,
) -> Result<(Vec<(String, String, Value)>, i32, i32), String> {
    use crate::data::backup::CharacterPluginMigration::{migrateCandidateTrigger, ownerRecord};
    let mut memories = BTreeMap::<i64, Value>::new();
    let mut links = Vec::new();
    let mut tags = BTreeMap::<i64, Value>::new();
    let mut properties = BTreeMap::<i64, Value>::new();
    let mut chunks = Vec::<(i64, Value)>::new();
    let mut candidates = Vec::new();
    let mut tag_relations = BTreeSet::new();
    let mut property_relations = BTreeSet::new();
    // Prefixes follow the checked-in Operit1 ObjectBox model entity and relation identifiers.
    visitLmdbRecords(storage, path, length, &mut |key, bytes| {
        if key.len() < 4 {
            return Ok(());
        }
        let prefix: [u8; 4] = key[0..4]
            .try_into()
            .map_err(|_| "Invalid ObjectBox key prefix")?;
        match (key.len(), prefix) {
            (8, OPERIT1_OBJECTBOX_KEY_MEMORY) if !bytes.is_empty() => {
                let memory = parseOperit1MemoryRecord(bytes)?;
                let table = FlatObjectBoxTable::new(bytes)?;
                let document = table
                    .optionalString(12)?
                    .map(|path| plan.rewritePath(&path))
                    .transpose()?;
                let index = table
                    .optionalString(14)?
                    .map(|path| plan.rewritePath(&path))
                    .transpose()?;
                let value = serde_json::json!({"id":memory.id,"uuid":memory.uuid,"title":memory.title,
                    "content":memory.content,"contentType":memory.contentType,"source":memory.source,
                    "credibility":memory.credibility,"importance":memory.importance,"folderPath":memory.folderPath,
                    "createdAt":memory.createdAt,"updatedAt":memory.updatedAt,"lastAccessedAt":table.requiredI64(10,"Memory.lastAccessedAt")?,
                    "documentPath":document,"isDocumentNode":memory.isDocumentNode,"chunkIndexFilePath":index,"tags":[],"properties":[]});
                if memories.insert(memory.id, value).is_some() {
                    return Err("Duplicate Operit1 memory identity".to_string());
                }
            }
            (8, OPERIT1_OBJECTBOX_KEY_LINK) if !bytes.is_empty() => {
                let link = parseOperit1MemoryLinkRecord(bytes)?;
                links.push(serde_json::json!({"id":link.id,"sourceMemoryId":link.sourceId,"targetMemoryId":link.targetId,
                    "type_":link.type_,"weight":link.weight,"description":link.description}));
            }
            (8, OPERIT1_OBJECTBOX_KEY_TAG) if !bytes.is_empty() => {
                let tag = parseOperit1MemoryTagRecord(bytes)?;
                tags.insert(tag.id, serde_json::json!({"id":tag.id,"name":tag.name}));
            }
            (8, [0x18, 0x00, 0x00, 0x18]) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                let id = table.requiredI64(0, "MemoryProperty.id")?;
                properties.insert(id, serde_json::json!({"id":id,"key":table.requiredString(1,"MemoryProperty.key")?,"value":table.requiredString(2,"MemoryProperty.value")?}));
            }
            (8, [0x18, 0x00, 0x00, 0x20]) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                let memory_id = table.requiredI64(4, "DocumentChunk.memoryId")?;
                chunks.push((memory_id, serde_json::json!({"id":table.requiredI64(0,"DocumentChunk.id")?,
                    "content":table.requiredString(1,"DocumentChunk.content")?,"chunkIndex":table.scalarI32(2,"DocumentChunk.chunkIndex")?})));
            }
            (8, [0x18, 0x00, 0x00, 0x2c]) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                let chat = table.requiredString(1, "MemoryAutoSaveCandidate.chatId")?;
                let timestamp =
                    table.requiredI64(2, "MemoryAutoSaveCandidate.triggerMessageTimestamp")?;
                let messages = archive
                    .chats
                    .iter()
                    .filter(|entry| entry.id == chat)
                    .flat_map(|entry| &entry.messages)
                    .map(|message| (
                        message.baseMessage.timestamp,
                        i64::from(message.baseMessage.selectedVariantIndex),
                        message.baseMessage.sender.clone(),
                    ))
                    .collect::<Vec<_>>();
                let mut candidate = serde_json::json!({"id":table.requiredI64(0,"MemoryAutoSaveCandidate.id")?,"chatId":chat,"triggerMessageTimestamp":timestamp,
                    "createdAt":table.requiredI64(3,"MemoryAutoSaveCandidate.createdAt")?,"updatedAt":table.requiredI64(4,"MemoryAutoSaveCandidate.updatedAt")?,"status":table.requiredString(5,"MemoryAutoSaveCandidate.status")?,"attemptCount":table.scalarI32(6,"MemoryAutoSaveCandidate.attemptCount")?,"lastError":table.requiredString(7,"MemoryAutoSaveCandidate.lastError")?,"sourceType":table.requiredString(8,"MemoryAutoSaveCandidate.sourceType")?});
                migrateCandidateTrigger(&mut candidate, &messages)?;
                candidates.push(candidate);
            }
            (16, OPERIT1_OBJECTBOX_KEY_MEMORY_TAG_RELATION) if bytes.is_empty() => {
                tag_relations.insert((
                    i64::from(readBigEndianU32(&key[8..12])?),
                    i64::from(readBigEndianU32(&key[12..16])?),
                ));
            }
            (16, [0x20, 0x00, 0x00, 0x24]) if bytes.is_empty() => {
                property_relations.insert((
                    i64::from(readBigEndianU32(&key[8..12])?),
                    i64::from(readBigEndianU32(&key[12..16])?),
                ));
            }
            _ => {}
        }
        Ok(())
    })?;
    for (memory_id, tag_id) in tag_relations {
        let tag = tags
            .get(&tag_id)
            .ok_or_else(|| format!("Missing Operit1 memory tag: {tag_id}"))?;
        let memory = memories
            .get_mut(&memory_id)
            .ok_or_else(|| format!("Missing Operit1 tagged memory: {memory_id}"))?;
        memory["tags"]
            .as_array_mut()
            .ok_or("Invalid memory tags")?
            .push(tag.clone());
    }
    for (memory_id, property_id) in property_relations {
        let property = properties
            .get(&property_id)
            .ok_or_else(|| format!("Missing Operit1 memory property: {property_id}"))?;
        let memory = memories
            .get_mut(&memory_id)
            .ok_or_else(|| format!("Missing Operit1 property owner: {memory_id}"))?;
        memory["properties"]
            .as_array_mut()
            .ok_or("Invalid memory properties")?
            .push(property.clone());
    }
    let memory_count = i32::try_from(memories.len()).map_err(|error| error.to_string())?;
    let link_count = i32::try_from(links.len()).map_err(|error| error.to_string())?;
    let mut records = Vec::new();
    for link in links {
        let source = link["sourceMemoryId"]
            .as_i64()
            .ok_or("Invalid source memory identity")?;
        let target = link["targetMemoryId"]
            .as_i64()
            .ok_or("Invalid target memory identity")?;
        if !memories.contains_key(&source) || !memories.contains_key(&target) {
            return Err("Operit1 relationship references missing memory".to_string());
        }
        records.push(ownerRecord("links", owner, link)?);
    }
    for (memory_id, mut chunk) in chunks {
        let memory = memories
            .get(&memory_id)
            .ok_or("Operit1 chunk references missing memory")?;
        if memory["isDocumentNode"] != true {
            return Err("Operit1 chunk references a non-document memory".to_string());
        }
        chunk["memoryUuid"] = memory["uuid"].clone();
        records.push(ownerRecord("chunks", owner, chunk)?);
    }
    for candidate in candidates {
        records.push(ownerRecord("candidates", owner, candidate)?);
    }
    for memory in memories.into_values() {
        records.push(ownerRecord("memories", owner, memory)?);
    }
    Ok((records, memory_count, link_count))
}

/// Reads the declared source layout for parseOperit1MemoryRecord.
#[allow(non_snake_case)]
fn parseOperit1MemoryRecord(bytes: &[u8]) -> Result<Operit1MemoryRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryRecord {
        id: table.requiredI64(0, "Memory.id")?,
        uuid: table.requiredString(1, "Memory.uuid")?,
        title: table.requiredString(2, "Memory.title")?,
        content: table.requiredString(3, "Memory.content")?,
        contentType: table.requiredString(4, "Memory.contentType")?,
        source: table.requiredString(5, "Memory.source")?,
        credibility: table.scalarF32(6, "Memory.credibility")?,
        importance: table.scalarF32(7, "Memory.importance")?,
        createdAt: table.requiredI64(8, "Memory.createdAt")?,
        updatedAt: table.requiredI64(9, "Memory.updatedAt")?,
        isDocumentNode: table.scalarBool(13)?,
        folderPath: table.optionalString(17)?,
    })
}

/// Reads the declared source layout for parseOperit1MemoryLinkRecord.
#[allow(non_snake_case)]
fn parseOperit1MemoryLinkRecord(bytes: &[u8]) -> Result<Operit1MemoryLinkRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryLinkRecord {
        id: table.requiredI64(0, "MemoryLink.id")?,
        type_: table.requiredString(1, "MemoryLink.type")?,
        weight: table.scalarF32(2, "MemoryLink.weight")?,
        description: table.requiredString(3, "MemoryLink.description")?,
        sourceId: table.requiredI64(6, "MemoryLink.sourceId")?,
        targetId: table.requiredI64(7, "MemoryLink.targetId")?,
    })
}

/// Reads the declared source layout for parseOperit1MemoryTagRecord.
#[allow(non_snake_case)]
fn parseOperit1MemoryTagRecord(bytes: &[u8]) -> Result<Operit1MemoryTagRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryTagRecord {
        id: table.requiredI64(0, "MemoryTag.id")?,
        name: table.requiredString(1, "MemoryTag.name")?,
    })
}

struct FlatObjectBoxTable<'a> {
    bytes: &'a [u8],
    tableStart: usize,
    offsets: Vec<usize>,
}

impl<'a> FlatObjectBoxTable<'a> {
    /// Reads the declared source layout for new.
    fn new(bytes: &'a [u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err("Operit1 ObjectBox 表内容过短".to_string());
        }
        let tableStart = readLittleEndianU32(&bytes[0..4])? as usize;
        if tableStart + 4 > bytes.len() {
            return Err("Operit1 ObjectBox 表根指针越界".to_string());
        }
        let vtableOffset = readLittleEndianI32(&bytes[tableStart..tableStart + 4])?;
        let vtableStart = if vtableOffset >= 0 {
            tableStart.checked_sub(vtableOffset as usize)
        } else {
            tableStart.checked_add(vtableOffset.unsigned_abs() as usize)
        }
        .ok_or_else(|| "Operit1 ObjectBox vtable 偏移无效".to_string())?;
        if vtableStart + 4 > bytes.len() {
            return Err("Operit1 ObjectBox vtable 越界".to_string());
        }
        let vtableLength = readLittleEndianU16(&bytes[vtableStart..vtableStart + 2])? as usize;
        if vtableLength < 4 || vtableLength % 2 != 0 || vtableStart + vtableLength > bytes.len() {
            return Err("Operit1 ObjectBox vtable 长度无效".to_string());
        }
        let fieldCount = (vtableLength - 4) / 2;
        let mut offsets = Vec::new();
        for fieldIndex in 0..fieldCount {
            let start = vtableStart + 4 + fieldIndex * 2;
            offsets.push(readLittleEndianU16(&bytes[start..start + 2])? as usize);
        }
        Ok(Self {
            bytes,
            tableStart,
            offsets,
        })
    }

    /// Reads the declared source layout for fieldAbs.
    #[allow(non_snake_case)]
    fn fieldAbs(&self, index: usize) -> Result<Option<usize>, String> {
        let Some(&relative) = self.offsets.get(index) else {
            return Ok(None);
        };
        if relative == 0 {
            return Ok(None);
        }
        let position = self
            .tableStart
            .checked_add(relative)
            .ok_or("ObjectBox field offset overflow")?;
        if position >= self.bytes.len() {
            return Err("ObjectBox field offset is outside the table".to_string());
        }
        Ok(Some(position))
    }

    /// Reads FlatBuffers' declared zero default for an omitted 32-bit scalar slot.
    #[allow(non_snake_case)]
    fn scalarI32(&self, index: usize, label: &str) -> Result<i32, String> {
        let Some(abs) = self.fieldAbs(index)? else {
            return Ok(0);
        };
        if abs + 4 > self.bytes.len() {
            return Err(format!("ObjectBox scalar is outside the table: {label}"));
        }
        readLittleEndianI32(&self.bytes[abs..abs + 4])
    }

    /// Reads the declared source layout for requiredI64.
    #[allow(non_snake_case)]
    fn requiredI64(&self, index: usize, label: &str) -> Result<i64, String> {
        let abs = self
            .fieldAbs(index)?
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))?;
        if abs + 8 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        readLittleEndianI64(&self.bytes[abs..abs + 8])
    }

    /// Reads the declared source layout for scalarF32.
    #[allow(non_snake_case)]
    fn scalarF32(&self, index: usize, label: &str) -> Result<f32, String> {
        let Some(abs) = self.fieldAbs(index)? else {
            return Ok(0.0);
        };
        if abs + 4 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        Ok(f32::from_le_bytes(
            self.bytes[abs..abs + 4]
                .try_into()
                .map_err(|_| format!("Operit1 ObjectBox 字段无效：{label}"))?,
        ))
    }

    /// Reads the declared source layout for requiredString.
    #[allow(non_snake_case)]
    fn requiredString(&self, index: usize, label: &str) -> Result<String, String> {
        self.optionalString(index)?
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))
    }

    /// Reads the declared source layout for optionalString.
    #[allow(non_snake_case)]
    fn optionalString(&self, index: usize) -> Result<Option<String>, String> {
        let Some(abs) = self.fieldAbs(index)? else {
            return Ok(None);
        };
        if abs + 4 > self.bytes.len() {
            return Err("Operit1 ObjectBox 字符串指针越界".to_string());
        }
        let relative = readLittleEndianI32(&self.bytes[abs..abs + 4])?;
        if relative <= 0 {
            return Err("Operit1 ObjectBox 字符串偏移无效".to_string());
        }
        let vectorStart = abs
            .checked_add(relative as usize)
            .ok_or_else(|| "Operit1 ObjectBox 字符串偏移溢出".to_string())?;
        if vectorStart + 4 > self.bytes.len() {
            return Err("Operit1 ObjectBox 字符串长度越界".to_string());
        }
        let length = readLittleEndianU32(&self.bytes[vectorStart..vectorStart + 4])? as usize;
        let start = vectorStart + 4;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "Operit1 ObjectBox 字符串长度溢出".to_string())?;
        if end > self.bytes.len() {
            return Err("Operit1 ObjectBox 字符串内容越界".to_string());
        }
        String::from_utf8(self.bytes[start..end].to_vec())
            .map(Some)
            .map_err(|error| error.to_string())
    }

    /// Reads the declared source layout for scalarBool.
    #[allow(non_snake_case)]
    fn scalarBool(&self, index: usize) -> Result<bool, String> {
        let Some(abs) = self.fieldAbs(index)? else {
            return Ok(false);
        };
        if abs >= self.bytes.len() {
            return Err("Operit1 ObjectBox 布尔字段越界".to_string());
        }
        Ok(self.bytes[abs] != 0)
    }
}

/// Reads the declared source layout for readBigEndianU32.
#[allow(non_snake_case)]
fn readBigEndianU32(bytes: &[u8]) -> Result<u32, String> {
    Ok(u32::from_be_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u32 字节长度无效".to_string()
    })?))
}

/// Reads the declared source layout for readLittleEndianU16.
#[allow(non_snake_case)]
fn readLittleEndianU16(bytes: &[u8]) -> Result<u16, String> {
    Ok(u16::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u16 字节长度无效".to_string()
    })?))
}

/// Reads the declared source layout for readLittleEndianU32.
#[allow(non_snake_case)]
fn readLittleEndianU32(bytes: &[u8]) -> Result<u32, String> {
    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u32 字节长度无效".to_string()
    })?))
}

/// Reads the declared source layout for readLittleEndianI32.
#[allow(non_snake_case)]
fn readLittleEndianI32(bytes: &[u8]) -> Result<i32, String> {
    Ok(i32::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox i32 字节长度无效".to_string()
    })?))
}

/// Reads the declared source layout for readLittleEndianI64.
#[allow(non_snake_case)]
fn readLittleEndianI64(bytes: &[u8]) -> Result<i64, String> {
    Ok(i64::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox i64 字节长度无效".to_string()
    })?))
}

/// Adapts a host-owned sequential storage session to the standard writer contract.
struct RuntimeStorageSessionWriter<'a> {
    session: &'a mut dyn RuntimeStorageWriteSession,
}

impl Write for RuntimeStorageSessionWriter<'_> {
    /// Writes one archive decoder chunk into the host-owned session.
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.session
            .writeChunk(buffer)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.message))?;
        Ok(buffer.len())
    }

    /// Flushes no additional state because each host chunk is synchronously accepted.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Streams one indexed archive entry into a host-owned runtime storage session.
fn writeArchiveEntryToStorage(
    storageWriteHost: &dyn RuntimeStorageWriteHost,
    parsed: &ParsedOperit1Snapshot,
    entry: &str,
    storagePath: &str,
) -> Result<(), String> {
    let mut session = storageWriteHost
        .createWriteSession(storagePath)
        .map_err(|error| error.to_string())?;
    let copyResult = parsed.copyEntryTo(
        entry,
        &mut RuntimeStorageSessionWriter {
            session: session.as_mut(),
        },
    );
    match copyResult {
        Ok(()) => session.commitFast().map_err(|error| error.to_string()),
        Err(error) => {
            session.discard().map_err(|discardError| {
                format!("{error}; failed to discard incomplete storage entry: {discardError}")
            })?;
            Err(error)
        }
    }
}

/// Streams one already-open archive entry into a host-owned runtime storage session.
fn writeArchiveReaderToStorage(
    storageWriteHost: &dyn RuntimeStorageWriteHost,
    reader: &mut dyn Read,
    storagePath: &str,
    buffer: &mut [u8],
) -> Result<(), String> {
    let mut session = storageWriteHost
        .createWriteSession(storagePath)
        .map_err(|error| error.to_string())?;
    let copyResult = copyArchiveReaderToStorageSession(
        reader,
        &mut RuntimeStorageSessionWriter {
            session: session.as_mut(),
        },
        buffer,
    );
    match copyResult {
        Ok(()) => session.commitFast().map_err(|error| error.to_string()),
        Err(error) => {
            session.discard().map_err(|discardError| {
                format!("{error}; failed to discard incomplete storage entry: {discardError}")
            })?;
            Err(error)
        }
    }
}

/// Copies one open archive reader into a storage session using the supplied reusable buffer.
fn copyArchiveReaderToStorageSession(
    reader: &mut dyn Read,
    writer: &mut RuntimeStorageSessionWriter<'_>,
    buffer: &mut [u8],
) -> Result<(), String> {
    loop {
        let count = reader.read(buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(());
        }
        writer
            .write_all(&buffer[..count])
            .map_err(|error| error.to_string())?;
    }
}
