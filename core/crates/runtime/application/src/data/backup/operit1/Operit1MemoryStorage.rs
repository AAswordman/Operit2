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

/// Recognizes legacy memory storage and vector indexes without excluding user workspace documents.
#[allow(non_snake_case)]
fn isOperit1LegacyMemoryEntry(entry: &str) -> bool {
    let Some(relative) = entry.strip_prefix(ENTRY_FILES_PREFIX) else {
        return false;
    };
    let Some((directory, file)) = relative.split_once('/') else {
        // RawSnapshotBackupManager also excludes the former top-level index filenames.
        return relative.starts_with("memory_hnsw_") && relative.ends_with(".idx")
            || relative.starts_with("doc_index_") && relative.ends_with(".hnsw");
    };
    (directory == "objectbox" || directory.starts_with("objectbox_")) && matches!(file, "data.mdb" | "lock.mdb")
        || matches!(directory, "memory-space-profiles" | ".vector_index")
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
        format!("/operitapp/workspaces/{workspaceId}")
    } else {
        format!("/operitapp/workspaces/{workspaceId}/{rest}")
    }
}

/// Joins a validated relative path to one stable virtual file-system root.
fn joinVirtualPath(root: &str, relative: &str) -> String {
    format!("{}/{relative}", root.trim_end_matches('/'))
}

/// Decodes forward entity/relation keys; index keys and reverse relations are not domain records.
fn decodeOperit1ObjectBoxKey(key: &[u8]) -> Result<Option<(u32, u32, Vec<i64>)>, String> {
    let Some(header) = key.get(..4) else { return Ok(None) };
    let prefix = u32::from_be_bytes(header.try_into().map_err(|_| "Invalid ObjectBox prefix")?);
    let kind = prefix >> 26;
    let model_id = (prefix >> 2) & 0x00ff_ffff;
    if !matches!((kind, model_id), (6, 4 | 5 | 6 | 7 | 8 | 11) | (2, 7 | 8))
        || prefix & 2 != 0
    {
        return Ok(None);
    }
    let width = 4 << (prefix & 1);
    let end = 4 + width;
    let widths = match (kind, key.len().checked_sub(end)) {
        (6, Some(0)) => vec![width],
        (2, Some(target_width @ (4 | 8))) => vec![width, target_width],
        _ => return Err("Invalid Operit1 ObjectBox key length".to_string()),
    };
    let mut offset = 4;
    let mut ids = Vec::new();
    for width in widths {
        let mut bytes = [0; 8];
        bytes[8 - width..].copy_from_slice(&key[offset..offset + width]);
        let id = i64::try_from(u64::from_be_bytes(bytes)).map_err(|_| "ObjectBox ID exceeds i64")?;
        if id <= 0 { return Err("Invalid Operit1 ObjectBox ID".to_string()); }
        ids.push(id);
        offset += width;
    }
    Ok(Some((kind, model_id, ids)))
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
    producerNodeId: &str,
) -> Result<(Vec<(String, String, Value)>, i32, i32), String> {
    use crate::data::backup::CharacterPluginMigration::{
        migrateCandidateTrigger, ownerRecord, CandidateSourceState,
    };
    let mut memories = BTreeMap::<i64, Value>::new();
    let mut links = Vec::new();
    let mut tags = BTreeMap::<i64, Value>::new();
    let mut properties = BTreeMap::<i64, Value>::new();
    let mut chunks = Vec::<(i64, Value)>::new();
    let mut candidates = Vec::new();
    let mut relations = BTreeSet::new();
    let mut dropped_candidates = 0usize;
    // Native ObjectBox encodes (kind << 26) | (model ID << 2) | width/direction flags.
    // Treating kind 8 indexes as kind 2 relations mislabels MemoryLink IDs as property IDs.
    visitLmdbRecords(storage, path, length, &mut |key, bytes| {
        let Some((kind, model_id, ids)) = decodeOperit1ObjectBoxKey(key)? else { return Ok(()) };
        match (kind, model_id) {
            (6, 4) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                let id = table.requiredI64(0, "Memory.id")?;
                let document = plan.rewriteMemoryAttachmentPath(table.optionalString(12)?.as_deref())?;
                // Operit1 excludes generated HNSW files from raw snapshots; retire their references.
                let value = serde_json::json!({"id":id,"uuid":table.requiredString(1,"Memory.uuid")?,
                    "title":table.requiredString(2,"Memory.title")?,"content":table.requiredString(3,"Memory.content")?,
                    "contentType":table.requiredString(4,"Memory.contentType")?,"source":table.requiredString(5,"Memory.source")?,
                    "credibility":table.scalarF32(6,"Memory.credibility")?,"importance":table.scalarF32(7,"Memory.importance")?,
                    "folderPath":table.optionalString(17)?,"createdAt":table.requiredI64(8,"Memory.createdAt")?,
                    "updatedAt":table.requiredI64(9,"Memory.updatedAt")?,"lastAccessedAt":table.requiredI64(10,"Memory.lastAccessedAt")?,
                    "documentPath":document,"isDocumentNode":table.scalarBool(13)?,"chunkIndexFilePath":null,"tags":[],"properties":[]});
                if memories.insert(id, value).is_some() {
                    return Err("Duplicate Operit1 memory identity".to_string());
                }
            }
            (6, 5) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                links.push(serde_json::json!({"id":table.requiredI64(0,"MemoryLink.id")?,
                    "sourceMemoryId":table.requiredI64(6,"MemoryLink.sourceId")?,"targetMemoryId":table.requiredI64(7,"MemoryLink.targetId")?,
                    "type_":table.requiredString(1,"MemoryLink.type")?,"weight":table.scalarF32(2,"MemoryLink.weight")?,
                    "description":table.requiredString(3,"MemoryLink.description")?}));
            }
            (6, 7) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                let id = table.requiredI64(0, "MemoryTag.id")?;
                tags.insert(id, serde_json::json!({"id":id,"name":table.requiredString(1,"MemoryTag.name")?}));
            }
            (6, 6) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                let id = table.requiredI64(0, "MemoryProperty.id")?;
                properties.insert(id, serde_json::json!({"id":id,"key":table.requiredString(1,"MemoryProperty.key")?,"value":table.requiredString(2,"MemoryProperty.value")?}));
            }
            (6, 8) if !bytes.is_empty() => {
                let table = FlatObjectBoxTable::new(bytes)?;
                let memory_id = table.requiredI64(4, "DocumentChunk.memoryId")?;
                chunks.push((memory_id, serde_json::json!({"id":table.requiredI64(0,"DocumentChunk.id")?,
                    "content":table.requiredString(1,"DocumentChunk.content")?,"chunkIndex":table.scalarI32(2,"DocumentChunk.chunkIndex")?})));
            }
            (6, 11) if !bytes.is_empty() => {
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
                let mut candidate = serde_json::json!({"id":table.requiredI64(0,"MemoryAutoSaveCandidate.id")?,"producerNodeId":producerNodeId,"chatId":chat,"triggerMessageTimestamp":timestamp,
                    "createdAt":table.requiredI64(3,"MemoryAutoSaveCandidate.createdAt")?,"updatedAt":table.requiredI64(4,"MemoryAutoSaveCandidate.updatedAt")?,"status":table.requiredString(5,"MemoryAutoSaveCandidate.status")?,"attemptCount":table.scalarI32(6,"MemoryAutoSaveCandidate.attemptCount")?,"lastError":table.requiredString(7,"MemoryAutoSaveCandidate.lastError")?,"sourceType":table.requiredString(8,"MemoryAutoSaveCandidate.sourceType")?});
                // The retired app deleted chats without clearing their queued extractions, and the
                // plugin requires a resolvable source revision, so stale queue records are dropped.
                if migrateCandidateTrigger(&mut candidate, &messages)?
                    == CandidateSourceState::Unresolvable
                {
                    dropped_candidates += 1;
                } else {
                    candidates.push(candidate);
                }
            }
            (2, relation_id) => {
                if !bytes.is_empty() { return Err("Invalid Operit1 ObjectBox relation value".to_string()); }
                relations.insert((if relation_id == 7 { "tags" } else { "properties" }, ids[0], ids[1]));
            }
            _ => {}
        }
        Ok(())
    })?;
    for (memory_field, memory_id, target_id) in relations {
        let targets = if memory_field == "tags" { &tags } else { &properties };
        let target = targets.get(&target_id)
            .ok_or_else(|| format!("Missing Operit1 {memory_field} target: {target_id}"))?;
        let memory = memories
            .get_mut(&memory_id)
            .ok_or_else(|| format!("Missing Operit1 {memory_field} owner: {memory_id}"))?;
        memory[memory_field]
            .as_array_mut()
            .ok_or("Invalid memory relation field")?
            .push(target.clone());
    }
    let memory_count = i32::try_from(memories.len()).map_err(|error| error.to_string())?;
    let mut records = Vec::new();
    let mut link_count = 0i32;
    let mut dropped_relationships = 0usize;
    let mut dropped_chunks = 0usize;
    for link in links {
        let source = link["sourceMemoryId"]
            .as_i64()
            .ok_or("Invalid source memory identity")?;
        let target = link["targetMemoryId"]
            .as_i64()
            .ok_or("Invalid target memory identity")?;
        // The retired app deleted memories without clearing their references, and the plugin rejects
        // unresolvable endpoints, so stale relationships are dropped rather than failing the import.
        if !memories.contains_key(&source) || !memories.contains_key(&target) {
            dropped_relationships += 1;
            continue;
        }
        records.push(ownerRecord("links", owner, link)?);
        link_count = link_count
            .checked_add(1)
            .ok_or("Operit1 relationship count overflow")?;
    }
    for (memory_id, mut chunk) in chunks {
        let Some(memory) = memories.get(&memory_id) else {
            dropped_chunks += 1;
            continue;
        };
        if memory["isDocumentNode"] != true {
            dropped_chunks += 1;
            continue;
        }
        chunk["memoryUuid"] = memory["uuid"].clone();
        records.push(ownerRecord("chunks", owner, chunk)?);
    }
    if dropped_relationships > 0 || dropped_chunks > 0 || dropped_candidates > 0 {
        use operit_util::AppLogger::AppLogger;
        AppLogger::w(
            "Operit1SnapshotImport",
            &format!(
                "owner={owner} dropped {dropped_relationships} relationships, {dropped_chunks} document chunks and {dropped_candidates} queued memory extractions whose legacy source was deleted"
            ),
        );
    }
    for candidate in candidates {
        records.push(ownerRecord("candidates", owner, candidate)?);
    }
    for memory in memories.into_values() {
        records.push(ownerRecord("memories", owner, memory)?);
    }
    Ok((records, memory_count, link_count))
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
