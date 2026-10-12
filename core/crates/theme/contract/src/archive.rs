use crate::{metadata_for_path, sha256_hex, validate_package_path, ThemeValidationError,
    ValidationStage, MAX_ARCHIVE_BYTES, MAX_COMPRESSION_RATIO, MAX_EXPANDED_BYTES,
    MAX_FILES};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use zip::{CompressionMethod, ZipArchive};

/// L0/L1 result: bytes are bounded, extracted in memory, and still need L2-L5.
#[derive(Debug)]
pub struct InspectedThemeArchive {
    pub archive_sha256: String,
    pub archive_bytes: usize,
    pub expanded_bytes: usize,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Inspects a ZIP without writing to disk. It rejects the archive before any
/// caller can turn it into an installable theme. `zip` is used only with its
/// stored/deflate readers and every produced byte remains under a hard budget.
pub fn inspect_theme_archive(
    archive_bytes: &[u8],
) -> Result<InspectedThemeArchive, ThemeValidationError> {
    if archive_bytes.is_empty() || archive_bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(archive_error("", "archive byte budget exceeded"));
    }
    // ZipArchive internally indexes by name. Read the raw directory first so
    // duplicate names, hidden local records and aliasing ranges cannot vanish
    // when that index is built.
    let entry_count = inspect_zip_records(archive_bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes))
        .map_err(|error| archive_error("", format!("invalid ZIP structure: {error}")))?;
    if archive.offset() != 0 || !archive.comment().is_empty() || archive.zip64_comment().is_some() {
        return Err(archive_error("", "prepended data, comments, and ZIP64 are not allowed"));
    }
    if archive.len() != entry_count {
        return Err(archive_error("", "ZIP entry count is outside the v1 budget"));
    }

    let mut files = BTreeMap::new();
    let mut expanded_bytes = 0usize;
    for index in 0..archive.len() {
        let (name, data) = {
            let mut file = archive.by_index(index).map_err(|error| {
                archive_error("", format!("cannot read ZIP entry {index}: {error}"))
            })?;
            let name = std::str::from_utf8(file.name_raw())
                .map_err(|_| archive_error("", "ZIP entry name is not UTF-8"))?
                .to_owned();
            validate_package_path(&name)
                .map_err(|error| archive_error(&name, error.message))?;
            if file.is_dir() || file.is_symlink() || file.encrypted() {
                return Err(archive_error(
                    &name,
                    "directories, symlinks, and encrypted entries are not allowed",
                ));
            }
            if let Some(mode) = file.unix_mode() {
                if mode & 0o170000 != 0 && mode & 0o170000 != 0o100000 {
                    return Err(archive_error(&name, "only regular files are allowed"));
                }
            }
            match file.compression() {
                CompressionMethod::Stored | CompressionMethod::Deflated => {}
                _ => return Err(archive_error(&name, "only Stored and Deflate are allowed")),
            }
            let size = usize::try_from(file.size())
                .map_err(|_| archive_error(&name, "ZIP entry size overflows usize"))?;
            let compressed = file.compressed_size();
            let (_, per_file_limit) = metadata_for_path(&name)
                .map_err(|error| archive_error(&name, error.message))?;
            if size == 0 || size > per_file_limit || size > MAX_EXPANDED_BYTES {
                return Err(archive_error(&name, "ZIP entry byte budget exceeded"));
            }
            if compressed == 0 || (size as u64) > compressed.saturating_mul(MAX_COMPRESSION_RATIO as u64) {
                return Err(archive_error(&name, "ZIP compression ratio exceeded"));
            }
            let mut data = Vec::with_capacity(size);
            (&mut file)
                .take((per_file_limit as u64).saturating_add(1))
                .read_to_end(&mut data)
                .map_err(|error| archive_error(&name, format!("cannot expand ZIP entry: {error}")))?;
            if data.len() != size {
                return Err(archive_error(&name, "expanded size differs from ZIP header"));
            }
            (name, data)
        };
        if files.insert(name.clone(), data).is_some() {
            return Err(archive_error(&name, "duplicate ZIP entry"));
        }
        expanded_bytes = expanded_bytes
            .checked_add(files[&name].len())
            .ok_or_else(|| archive_error(&name, "expanded byte count overflow"))?;
        if expanded_bytes > MAX_EXPANDED_BYTES {
            return Err(archive_error(&name, "expanded archive budget exceeded"));
        }
    }
    Ok(InspectedThemeArchive {
        archive_sha256: sha256_hex(archive_bytes),
        archive_bytes: archive_bytes.len(),
        expanded_bytes,
        files,
    })
}

/// v1 ZIP profile: a single disk, no ZIP64, extras/comments/data descriptors,
/// exact matching local/central names and metadata, and contiguous registered
/// file records. This also excludes bytes appended after the EOCD.
fn inspect_zip_records(bytes: &[u8]) -> Result<usize, ThemeValidationError> {
    let end = bytes.len().checked_sub(22)
        .ok_or_else(|| archive_error("", "ZIP EOCD is truncated"))?;
    if bytes.get(end..end + 4) != Some(b"PK\x05\x06".as_slice())
        || le16(bytes, end + 4)? != 0 || le16(bytes, end + 6)? != 0
        || le16(bytes, end + 20)? != 0
    {
        return Err(archive_error("", "ZIP must end at a single-disk, comment-free EOCD"));
    }
    let count = le16(bytes, end + 10)? as usize;
    let directory_size = le32(bytes, end + 12)? as usize;
    let directory_start = le32(bytes, end + 16)? as usize;
    if count == 0 || count > MAX_FILES || le16(bytes, end + 8)? as usize != count
        || directory_size == u32::MAX as usize || directory_start == u32::MAX as usize
        || directory_start.checked_add(directory_size) != Some(end)
    {
        return Err(archive_error("", "ZIP directory bounds, count or disk fields are invalid"));
    }
    let mut position = directory_start;
    let mut names = BTreeSet::new();
    let mut ranges = Vec::with_capacity(count);
    let mut total = 0usize;
    for _ in 0..count {
        if position.checked_add(46).is_none() || position + 46 > end
            || bytes.get(position..position + 4) != Some(b"PK\x01\x02".as_slice())
        {
            return Err(archive_error("", "ZIP central record is truncated"));
        }
        let flags = le16(bytes, position + 8)?;
        let method = le16(bytes, position + 10)?;
        let compressed = le32(bytes, position + 20)? as usize;
        let expanded = le32(bytes, position + 24)? as usize;
        let name_len = le16(bytes, position + 28)? as usize;
        let extra_len = le16(bytes, position + 30)? as usize;
        let comment_len = le16(bytes, position + 32)? as usize;
        let internal_attributes = le16(bytes, position + 36)?;
        let external_attributes = le32(bytes, position + 38)?;
        let local = le32(bytes, position + 42)? as usize;
        let next = position.checked_add(46 + name_len + extra_len + comment_len)
            .ok_or_else(|| archive_error("", "ZIP directory record length overflows usize"))?;
        if next > end || le16(bytes, position + 6)? > 20
            || flags & !0x0806 != 0 || !matches!(method, 0 | 8)
            || extra_len != 0 || comment_len != 0 || internal_attributes != 0
        {
            return Err(archive_error("", "ZIP entry flags, version, compression or metadata are outside the v1 profile"));
        }
        let name = std::str::from_utf8(&bytes[position + 46..position + 46 + name_len])
            .map_err(|_| archive_error("", "ZIP name is not UTF-8"))?;
        let made_by_system = le16(bytes, position + 4)? >> 8;
        if made_by_system == 3 {
            let mode = external_attributes >> 16;
            if mode != 0 && mode & 0o170000 != 0 && mode & 0o170000 != 0o100000 {
                return Err(archive_error(name, "ZIP external attributes declare a non-regular Unix file"));
            }
        } else if external_attributes & 0x10 != 0 {
            return Err(archive_error(name, "ZIP external attributes declare a directory"));
        }
        validate_package_path(name).map_err(|error| archive_error(name, error.message))?;
        if !names.insert(name) {
            return Err(archive_error(name, "duplicate raw ZIP name"));
        }
        let (_, limit) = metadata_for_path(name).map_err(|error| archive_error(name, error.message))?;
        total = total.checked_add(expanded).ok_or_else(|| archive_error(name, "ZIP expanded count overflows usize"))?;
        if expanded == 0 || expanded > limit || total > MAX_EXPANDED_BYTES
            || compressed == 0 || expanded as u64 > compressed as u64 * MAX_COMPRESSION_RATIO as u64
            || (method == 0 && compressed != expanded)
        {
            return Err(archive_error(name, "ZIP declared size or compression budget exceeded"));
        }
        if local.checked_add(30).is_none() || local + 30 > directory_start
            || bytes.get(local..local + 4) != Some(b"PK\x03\x04".as_slice())
        {
            return Err(archive_error(name, "ZIP local header is outside the file area"));
        }
        let local_name_len = le16(bytes, local + 26)? as usize;
        let local_extra_len = le16(bytes, local + 28)? as usize;
        let data_start = local.checked_add(30 + local_name_len + local_extra_len)
            .ok_or_else(|| archive_error(name, "ZIP local name range overflows usize"))?;
        let data_end = data_start.checked_add(compressed)
            .ok_or_else(|| archive_error(name, "ZIP compressed range overflows usize"))?;
        if data_end > directory_start || local_extra_len != 0 || local_name_len != name_len
            || bytes[local + 30..data_start] != *name.as_bytes()
            // version, flags, method, time/date, CRC, sizes
            || bytes[local + 4..local + 26] != bytes[position + 6..position + 28]
        {
            return Err(archive_error(name, "ZIP local and central records differ"));
        }
        ranges.push((local, data_end));
        position = next;
    }
    if position != end {
        return Err(archive_error("", "ZIP directory contains unregistered records"));
    }
    ranges.sort_unstable();
    let mut next_local = 0usize;
    for (start, finish) in ranges {
        if start != next_local {
            return Err(archive_error("", "ZIP contains prepended, hidden or overlapping local records"));
        }
        next_local = finish;
    }
    if next_local != directory_start {
        return Err(archive_error("", "ZIP contains bytes outside registered records"));
    }
    Ok(count)
}

fn le16(bytes: &[u8], offset: usize) -> Result<u16, ThemeValidationError> {
    let value = bytes.get(offset..offset.saturating_add(2))
        .ok_or_else(|| archive_error("", "ZIP field is truncated"))?;
    Ok(u16::from_le_bytes(value.try_into().map_err(|_| archive_error("", "ZIP field is truncated"))?))
}

fn le32(bytes: &[u8], offset: usize) -> Result<u32, ThemeValidationError> {
    let value = bytes.get(offset..offset.saturating_add(4))
        .ok_or_else(|| archive_error("", "ZIP field is truncated"))?;
    Ok(u32::from_le_bytes(value.try_into().map_err(|_| archive_error("", "ZIP field is truncated"))?))
}

fn archive_error(path: &str, message: impl Into<String>) -> ThemeValidationError {
    ThemeValidationError::new(
        "theme-package-archive-invalid",
        ValidationStage::Archive,
        path,
        message,
    )
}
