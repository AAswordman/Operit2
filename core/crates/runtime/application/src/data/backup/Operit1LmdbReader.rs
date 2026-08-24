use std::collections::HashSet;

use operit_host_api::RuntimeStorageHost;

const LMDB_MAGIC: u32 = 0xBEEFC0DE;
const LMDB_VERSION: u32 = 1;
const PAGE_BRANCH: u16 = 0x01;
const PAGE_LEAF: u16 = 0x02;
const PAGE_OVERFLOW: u16 = 0x04;
const PAGE_LEAF2: u16 = 0x20;
const NODE_BIG_DATA: u16 = 0x01;
const MAX_LMDB_RECORD_BYTES: usize = 32 * 1024 * 1024;
const MAX_LMDB_RECORDS: usize = 2_000_000;
const PAGE_SIZE_CANDIDATES: [usize; 7] = [1024, 2048, 4096, 8192, 16384, 32768, 65536];

/// Describes the native word layout used by one legacy LMDB environment.
#[derive(Clone, Copy)]
struct LmdbLayout {
    pageSize: usize,
    pointerSize: usize,
    pageHeaderSize: usize,
    flagsOffset: usize,
    lowerOffset: usize,
    upperOffset: usize,
    rootOffset: usize,
    lastPageOffset: usize,
    transactionIdOffset: usize,
}

/// Visits every live key/value record in an LMDB default database.
pub(crate) fn visitLmdbRecords(
    storageHost: &dyn RuntimeStorageHost,
    storagePath: &str,
    byteLength: u64,
    visitor: &mut dyn FnMut(&[u8], &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let layout = detectLayout(storageHost, storagePath, byteLength)?;
    let rootPage = activeRootPage(storageHost, storagePath, byteLength, layout)?;
    if rootPage == invalidPageNumber(layout.pointerSize) {
        return Ok(());
    }
    let pageCount = byteLength / layout.pageSize as u64;
    let mut pending = vec![rootPage];
    let mut visited = HashSet::new();
    let mut recordCount = 0usize;
    while let Some(pageNumber) = pending.pop() {
        if pageNumber >= pageCount {
            return Err(format!(
                "Operit1 LMDB page is outside the data file: {pageNumber}"
            ));
        }
        if !visited.insert(pageNumber) {
            continue;
        }
        let page = readExactRange(
            storageHost,
            storagePath,
            pageNumber * layout.pageSize as u64,
            layout.pageSize,
        )?;
        validatePageNumber(&page, pageNumber, layout)?;
        let flags = readU16(&page, layout.flagsOffset, "LMDB page flags")?;
        if flags & PAGE_LEAF2 != 0 {
            return Err("Operit1 LMDB fixed-key leaf pages are unsupported".to_string());
        }
        if flags & PAGE_BRANCH != 0 {
            for offset in nodeOffsets(&page, layout)? {
                let node = nodeHeader(&page, offset)?;
                pending.push(u64::from(node.dataSize));
            }
            continue;
        }
        if flags & PAGE_LEAF == 0 {
            return Err(format!(
                "Operit1 LMDB tree contains an invalid page type: {flags}"
            ));
        }
        for offset in nodeOffsets(&page, layout)? {
            recordCount = recordCount
                .checked_add(1)
                .ok_or_else(|| "Operit1 LMDB record count overflowed".to_string())?;
            if recordCount > MAX_LMDB_RECORDS {
                return Err(format!(
                    "Operit1 LMDB contains more than {MAX_LMDB_RECORDS} records"
                ));
            }
            let node = nodeHeader(&page, offset)?;
            let keyStart = offset + 8;
            let keyEnd = keyStart
                .checked_add(node.keySize)
                .ok_or_else(|| "Operit1 LMDB key range overflowed".to_string())?;
            let dataStart = keyStart
                .checked_add(alignEven(node.keySize))
                .ok_or_else(|| "Operit1 LMDB value range overflowed".to_string())?;
            let key = page
                .get(keyStart..keyEnd)
                .ok_or_else(|| "Operit1 LMDB key is truncated".to_string())?;
            let value = if node.flags & NODE_BIG_DATA != 0 {
                readOverflowValue(
                    storageHost,
                    storagePath,
                    byteLength,
                    &page,
                    dataStart,
                    node.dataSize as usize,
                    layout,
                )?
            } else {
                let dataEnd = dataStart
                    .checked_add(node.dataSize as usize)
                    .ok_or_else(|| "Operit1 LMDB value range overflowed".to_string())?;
                page.get(dataStart..dataEnd)
                    .ok_or_else(|| "Operit1 LMDB value is truncated".to_string())?
                    .to_vec()
            };
            visitor(key, &value)?;
        }
    }
    Ok(())
}

/// Detects page size and native word width from the two LMDB metadata pages.
fn detectLayout(
    storageHost: &dyn RuntimeStorageHost,
    storagePath: &str,
    byteLength: u64,
) -> Result<LmdbLayout, String> {
    for pointerSize in [8usize, 4usize] {
        let pageHeaderSize = if pointerSize == 8 { 16 } else { 12 };
        for pageSize in PAGE_SIZE_CANDIDATES {
            if byteLength < (pageSize * 2) as u64 {
                continue;
            }
            let firstMagic = readExactRange(storageHost, storagePath, pageHeaderSize as u64, 8)?;
            let secondMagic = readExactRange(
                storageHost,
                storagePath,
                pageSize as u64 + pageHeaderSize as u64,
                8,
            )?;
            if readU32(&firstMagic, 0, "LMDB magic")? != LMDB_MAGIC
                || readU32(&secondMagic, 0, "LMDB magic")? != LMDB_MAGIC
                || readU32(&firstMagic, 4, "LMDB version")? != LMDB_VERSION
                || readU32(&secondMagic, 4, "LMDB version")? != LMDB_VERSION
            {
                continue;
            }
            let (rootOffset, lastPageOffset, transactionIdOffset) = if pointerSize == 8 {
                (128, 136, 144)
            } else {
                (80, 84, 88)
            };
            return Ok(LmdbLayout {
                pageSize,
                pointerSize,
                pageHeaderSize,
                flagsOffset: if pointerSize == 8 { 10 } else { 6 },
                lowerOffset: if pointerSize == 8 { 12 } else { 8 },
                upperOffset: if pointerSize == 8 { 14 } else { 10 },
                rootOffset,
                lastPageOffset,
                transactionIdOffset,
            });
        }
    }
    Err("Operit1 ObjectBox data is not a supported LMDB environment".to_string())
}

/// Selects the newest valid LMDB metadata page and returns its main-database root.
fn activeRootPage(
    storageHost: &dyn RuntimeStorageHost,
    storagePath: &str,
    byteLength: u64,
    layout: LmdbLayout,
) -> Result<u64, String> {
    let pageCount = byteLength / layout.pageSize as u64;
    let mut selected: Option<(u64, u64)> = None;
    for metaIndex in 0..2u64 {
        let page = readExactRange(
            storageHost,
            storagePath,
            metaIndex * layout.pageSize as u64,
            layout.pageSize,
        )?;
        let lastPage = readWord(&page, layout.lastPageOffset, layout.pointerSize)?;
        let transactionId = readWord(&page, layout.transactionIdOffset, layout.pointerSize)?;
        let rootPage = readWord(&page, layout.rootOffset, layout.pointerSize)?;
        if lastPage >= pageCount
            || (rootPage != invalidPageNumber(layout.pointerSize) && rootPage > lastPage)
        {
            continue;
        }
        if selected.is_none_or(|(currentTransactionId, _)| transactionId > currentTransactionId) {
            selected = Some((transactionId, rootPage));
        }
    }
    selected
        .map(|(_, rootPage)| rootPage)
        .ok_or_else(|| "Operit1 LMDB metadata pages are invalid".to_string())
}

/// Stores the decoded fixed-size portion of one LMDB node.
struct LmdbNodeHeader {
    dataSize: u32,
    flags: u16,
    keySize: usize,
}

/// Reads and validates one LMDB node header.
fn nodeHeader(page: &[u8], offset: usize) -> Result<LmdbNodeHeader, String> {
    Ok(LmdbNodeHeader {
        dataSize: readU32(page, offset, "LMDB node data length")?,
        flags: readU16(page, offset + 4, "LMDB node flags")?,
        keySize: readU16(page, offset + 6, "LMDB node key length")? as usize,
    })
}

/// Reads the node-offset table from a branch or leaf page.
fn nodeOffsets(page: &[u8], layout: LmdbLayout) -> Result<Vec<usize>, String> {
    let lower = readU16(page, layout.lowerOffset, "LMDB page lower bound")? as usize;
    let upper = readU16(page, layout.upperOffset, "LMDB page upper bound")? as usize;
    if lower < layout.pageHeaderSize
        || lower > layout.pageSize
        || upper < lower
        || upper > layout.pageSize
        || (lower - layout.pageHeaderSize) % 2 != 0
    {
        return Err("Operit1 LMDB page bounds are invalid".to_string());
    }
    let mut offsets = Vec::with_capacity((lower - layout.pageHeaderSize) / 2);
    for position in (layout.pageHeaderSize..lower).step_by(2) {
        let offset = readU16(page, position, "LMDB node offset")? as usize;
        if offset < upper || offset + 8 > layout.pageSize {
            return Err("Operit1 LMDB node offset is invalid".to_string());
        }
        offsets.push(offset);
    }
    Ok(offsets)
}

/// Reads one overflow-backed value from its contiguous LMDB page span.
fn readOverflowValue(
    storageHost: &dyn RuntimeStorageHost,
    storagePath: &str,
    byteLength: u64,
    leafPage: &[u8],
    referenceOffset: usize,
    valueLength: usize,
    layout: LmdbLayout,
) -> Result<Vec<u8>, String> {
    if valueLength > MAX_LMDB_RECORD_BYTES {
        return Err(format!(
            "Operit1 LMDB record exceeds the {MAX_LMDB_RECORD_BYTES}-byte limit"
        ));
    }
    let overflowPage = readWord(leafPage, referenceOffset, layout.pointerSize)?;
    let pageOffset = overflowPage
        .checked_mul(layout.pageSize as u64)
        .ok_or_else(|| "Operit1 LMDB overflow offset overflowed".to_string())?;
    let header = readExactRange(storageHost, storagePath, pageOffset, layout.pageHeaderSize)?;
    let flags = readU16(&header, layout.flagsOffset, "LMDB overflow flags")?;
    if flags & PAGE_OVERFLOW == 0 {
        return Err("Operit1 LMDB big-data node does not reference an overflow page".to_string());
    }
    let pages = readU32(&header, layout.lowerOffset, "LMDB overflow page count")? as u64;
    let capacity = pages
        .checked_mul(layout.pageSize as u64)
        .and_then(|total| total.checked_sub(layout.pageHeaderSize as u64))
        .ok_or_else(|| "Operit1 LMDB overflow capacity overflowed".to_string())?;
    let end = pageOffset
        .checked_add(layout.pageHeaderSize as u64)
        .and_then(|start| start.checked_add(valueLength as u64))
        .ok_or_else(|| "Operit1 LMDB overflow range overflowed".to_string())?;
    if valueLength as u64 > capacity || end > byteLength {
        return Err("Operit1 LMDB overflow value is truncated".to_string());
    }
    readExactRange(
        storageHost,
        storagePath,
        pageOffset + layout.pageHeaderSize as u64,
        valueLength,
    )
}

/// Verifies that a decoded LMDB page header matches its requested page number.
fn validatePageNumber(page: &[u8], expected: u64, layout: LmdbLayout) -> Result<(), String> {
    let actual = readWord(page, 0, layout.pointerSize)?;
    if actual != expected {
        return Err(format!(
            "Operit1 LMDB page number mismatch: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

/// Reads one exact storage range and rejects truncated host responses.
fn readExactRange(
    storageHost: &dyn RuntimeStorageHost,
    storagePath: &str,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>, String> {
    let bytes = storageHost
        .readBytesRange(storagePath, offset, length)
        .map_err(|error| error.to_string())?;
    if bytes.len() != length {
        return Err(format!(
            "Operit1 LMDB range is truncated: expected {length} bytes, got {}",
            bytes.len()
        ));
    }
    Ok(bytes)
}

/// Reads one native-width little-endian LMDB word.
fn readWord(bytes: &[u8], offset: usize, pointerSize: usize) -> Result<u64, String> {
    if pointerSize == 8 {
        let value = bytes
            .get(offset..offset + 8)
            .ok_or_else(|| "Operit1 LMDB native word is truncated".to_string())?;
        return Ok(u64::from_le_bytes(
            value
                .try_into()
                .map_err(|_| "Operit1 LMDB native word is invalid".to_string())?,
        ));
    }
    Ok(u64::from(readU32(bytes, offset, "LMDB native word")?))
}

/// Reads one little-endian 16-bit LMDB field.
fn readU16(bytes: &[u8], offset: usize, label: &str) -> Result<u16, String> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| format!("Operit1 {label} is truncated"))?;
    Ok(u16::from_le_bytes(
        value
            .try_into()
            .map_err(|_| format!("Operit1 {label} is invalid"))?,
    ))
}

/// Reads one little-endian 32-bit LMDB field.
fn readU32(bytes: &[u8], offset: usize, label: &str) -> Result<u32, String> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| format!("Operit1 {label} is truncated"))?;
    Ok(u32::from_le_bytes(
        value
            .try_into()
            .map_err(|_| format!("Operit1 {label} is invalid"))?,
    ))
}

/// Rounds one LMDB key length up to its two-byte node alignment.
fn alignEven(value: usize) -> usize {
    (value + 1) & !1
}

/// Returns the all-ones page sentinel for one native LMDB word width.
fn invalidPageNumber(pointerSize: usize) -> u64 {
    if pointerSize == 8 {
        u64::MAX
    } else {
        u64::from(u32::MAX)
    }
}
