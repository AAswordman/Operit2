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

/// Returns whether an archive entry belongs to one non-workspace copy prefix.
fn entryMatchesCopyPrefix(entry: &str, sourcePrefix: &str) -> bool {
    entry.starts_with(sourcePrefix)
        && !isDataStoreEntry(entry)
        && !(sourcePrefix == ENTRY_FILES_PREFIX && entry.starts_with(ENTRY_WORKSPACE_FILES_PREFIX))
}

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
    id: u32,
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
    id: u32,
    type_: String,
    weight: f32,
    description: String,
    sourceId: u32,
    targetId: u32,
}

#[derive(Clone, Debug)]
struct Operit1MemoryTagRecord {
    id: u32,
    name: String,
}

#[allow(non_snake_case)]
fn buildMemoryExportDataFromOperit1ObjectBox(
    storageHost: &dyn RuntimeStorageHost,
    storagePath: &str,
    byteLength: u64,
) -> Result<MemoryExportData, String> {
    let mut memories = BTreeMap::<u32, Operit1MemoryRecord>::new();
    let mut links = BTreeMap::<u32, Operit1MemoryLinkRecord>::new();
    let mut tags = BTreeMap::<u32, Operit1MemoryTagRecord>::new();
    let mut memoryTagPairs = BTreeSet::<(u32, u32)>::new();

    visitLmdbRecords(storageHost, storagePath, byteLength, &mut |key, value| {
        if key.len() == 8 && key[0..4] == OPERIT1_OBJECTBOX_KEY_MEMORY && !value.is_empty() {
            let memory = parseOperit1MemoryRecord(value)?;
            memories.insert(memory.id, memory);
        } else if key.len() == 8 && key[0..4] == OPERIT1_OBJECTBOX_KEY_LINK && !value.is_empty() {
            let link = parseOperit1MemoryLinkRecord(value)?;
            links.insert(link.id, link);
        } else if key.len() == 8 && key[0..4] == OPERIT1_OBJECTBOX_KEY_TAG && !value.is_empty() {
            let tag = parseOperit1MemoryTagRecord(value)?;
            tags.insert(tag.id, tag);
        } else if key.len() == 16
            && key[0..4] == OPERIT1_OBJECTBOX_KEY_MEMORY_TAG_RELATION
            && value.is_empty()
        {
            let memoryId = readBigEndianU32(&key[8..12])?;
            let tagId = readBigEndianU32(&key[12..16])?;
            memoryTagPairs.insert((memoryId, tagId));
        }
        Ok(())
    })?;

    let mut tagNamesByMemoryId = HashMap::<u32, Vec<String>>::new();
    for (memoryId, tagId) in memoryTagPairs {
        let tag = tags
            .get(&tagId)
            .ok_or_else(|| format!("Operit1 记忆标签关系引用不存在的标签：{tagId}"))?;
        tagNamesByMemoryId
            .entry(memoryId)
            .or_default()
            .push(tag.name.clone());
    }
    for tagNames in tagNamesByMemoryId.values_mut() {
        tagNames.sort();
        tagNames.dedup();
    }

    let mut memoryUuidById = HashMap::<u32, String>::new();
    let mut serializableMemories = Vec::new();
    for memory in memories.values() {
        if memory.isDocumentNode {
            continue;
        }
        memoryUuidById.insert(memory.id, memory.uuid.clone());
        serializableMemories.push(SerializableMemory {
            uuid: memory.uuid.clone(),
            title: memory.title.clone(),
            content: memory.content.clone(),
            contentType: memory.contentType.clone(),
            source: memory.source.clone(),
            credibility: memory.credibility,
            importance: memory.importance,
            folderPath: memory.folderPath.clone(),
            createdAt: memory.createdAt,
            updatedAt: memory.updatedAt,
            tagNames: tagNamesByMemoryId.remove(&memory.id).unwrap_or_default(),
        });
    }

    let mut seenLinks = BTreeSet::new();
    let mut serializableLinks = Vec::new();
    for link in links.values() {
        let sourceUuid = memoryUuidById
            .get(&link.sourceId)
            .ok_or_else(|| format!("Operit1 记忆链接引用不存在的源记忆：{}", link.sourceId))?;
        let targetUuid = memoryUuidById
            .get(&link.targetId)
            .ok_or_else(|| format!("Operit1 记忆链接引用不存在的目标记忆：{}", link.targetId))?;
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
            type_: link.type_.clone(),
            weight: link.weight,
            description: link.description.clone(),
        });
    }

    Ok(MemoryExportData {
        memories: serializableMemories,
        links: serializableLinks,
        exportDate: currentTimeMillis(),
        version: "1.0".to_string(),
    })
}

#[allow(non_snake_case)]
fn parseOperit1MemoryRecord(bytes: &[u8]) -> Result<Operit1MemoryRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryRecord {
        id: table.requiredU32(0, "Memory.id")?,
        uuid: table.requiredString(1, "Memory.uuid")?,
        title: table.requiredString(2, "Memory.title")?,
        content: table.requiredString(3, "Memory.content")?,
        contentType: table.requiredString(4, "Memory.contentType")?,
        source: table.requiredString(5, "Memory.source")?,
        credibility: table.requiredF32(6, "Memory.credibility")?,
        importance: table.requiredF32(7, "Memory.importance")?,
        createdAt: table.requiredI64(8, "Memory.createdAt")?,
        updatedAt: table.requiredI64(9, "Memory.updatedAt")?,
        isDocumentNode: table.optionalBool(13)?.unwrap_or(false),
        folderPath: table.optionalString(17)?,
    })
}

#[allow(non_snake_case)]
fn parseOperit1MemoryLinkRecord(bytes: &[u8]) -> Result<Operit1MemoryLinkRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryLinkRecord {
        id: table.requiredU32(0, "MemoryLink.id")?,
        type_: table.requiredString(1, "MemoryLink.type")?,
        weight: table.requiredF32(2, "MemoryLink.weight")?,
        description: table.requiredString(3, "MemoryLink.description")?,
        sourceId: table.requiredU32(6, "MemoryLink.sourceId")?,
        targetId: table.requiredU32(7, "MemoryLink.targetId")?,
    })
}

#[allow(non_snake_case)]
fn parseOperit1MemoryTagRecord(bytes: &[u8]) -> Result<Operit1MemoryTagRecord, String> {
    let table = FlatObjectBoxTable::new(bytes)?;
    Ok(Operit1MemoryTagRecord {
        id: table.requiredU32(0, "MemoryTag.id")?,
        name: table.requiredString(1, "MemoryTag.name")?,
    })
}

struct FlatObjectBoxTable<'a> {
    bytes: &'a [u8],
    tableStart: usize,
    offsets: Vec<usize>,
}

impl<'a> FlatObjectBoxTable<'a> {
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
            tableStart.checked_add((-vtableOffset) as usize)
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

    #[allow(non_snake_case)]
    fn fieldAbs(&self, index: usize) -> Option<usize> {
        let relative = *self.offsets.get(index)?;
        if relative == 0 {
            return None;
        }
        self.tableStart
            .checked_add(relative)
            .filter(|position| *position < self.bytes.len())
    }

    #[allow(non_snake_case)]
    fn requiredU32(&self, index: usize, label: &str) -> Result<u32, String> {
        let abs = self
            .fieldAbs(index)
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))?;
        self.readU32Abs(abs, label)
    }

    #[allow(non_snake_case)]
    fn requiredI64(&self, index: usize, label: &str) -> Result<i64, String> {
        let abs = self
            .fieldAbs(index)
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))?;
        if abs + 8 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        readLittleEndianI64(&self.bytes[abs..abs + 8])
    }

    #[allow(non_snake_case)]
    fn requiredF32(&self, index: usize, label: &str) -> Result<f32, String> {
        let abs = self
            .fieldAbs(index)
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))?;
        if abs + 4 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        Ok(f32::from_le_bytes(
            self.bytes[abs..abs + 4]
                .try_into()
                .map_err(|_| format!("Operit1 ObjectBox 字段无效：{label}"))?,
        ))
    }

    #[allow(non_snake_case)]
    fn requiredString(&self, index: usize, label: &str) -> Result<String, String> {
        self.optionalString(index)?
            .ok_or_else(|| format!("Operit1 ObjectBox 字段缺失：{label}"))
    }

    #[allow(non_snake_case)]
    fn optionalString(&self, index: usize) -> Result<Option<String>, String> {
        let Some(abs) = self.fieldAbs(index) else {
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

    #[allow(non_snake_case)]
    fn optionalBool(&self, index: usize) -> Result<Option<bool>, String> {
        let Some(abs) = self.fieldAbs(index) else {
            return Ok(None);
        };
        if abs >= self.bytes.len() {
            return Err("Operit1 ObjectBox 布尔字段越界".to_string());
        }
        Ok(Some(self.bytes[abs] != 0))
    }

    #[allow(non_snake_case)]
    fn readU32Abs(&self, abs: usize, label: &str) -> Result<u32, String> {
        if abs + 4 > self.bytes.len() {
            return Err(format!("Operit1 ObjectBox 字段越界：{label}"));
        }
        readLittleEndianU32(&self.bytes[abs..abs + 4])
    }
}

#[allow(non_snake_case)]
fn readBigEndianU32(bytes: &[u8]) -> Result<u32, String> {
    Ok(u32::from_be_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u32 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn readLittleEndianU16(bytes: &[u8]) -> Result<u16, String> {
    Ok(u16::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u16 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn readLittleEndianU32(bytes: &[u8]) -> Result<u32, String> {
    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox u32 字节长度无效".to_string()
    })?))
}

#[allow(non_snake_case)]
fn readLittleEndianI32(bytes: &[u8]) -> Result<i32, String> {
    Ok(i32::from_le_bytes(bytes.try_into().map_err(|_| {
        "Operit1 ObjectBox i32 字节长度无效".to_string()
    })?))
}

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

