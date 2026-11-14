//! Shared streaming ZIP export, manifest validation, and authenticated payload reads.
use std::collections::BTreeSet;
use std::io::{Read, Seek, Write};

use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    payload_bytes, text, validate_files, validate_source_files, RescueFile, RescueManifest,
    RescueProgress, CHUNK_BYTES, MAX_FILES, MAX_METADATA_BYTES,
};

/// Streams a portable archive with plaintext preference exports and per-file SHA-256.
pub fn write_archive<W: Write + Seek>(
    output: W,
    layout: &str,
    metadata: Value,
    mut files: Vec<RescueFile>,
    mut read: impl FnMut(&str, u64, usize) -> Result<Vec<u8>, String>,
    mut preferences: impl FnMut(&RescueFile) -> Result<Option<Vec<u8>>, String>,
    mut report: impl FnMut(RescueProgress),
) -> Result<RescueManifest, String> {
    files.sort_by(|a, b| a.path.cmp(&b.path));
    validate_source_files(&files)?;
    let mut zip = ZipWriter::new(output);
    let options = SimpleFileOptions::default()
        .large_file(true)
        .compression_method(CompressionMethod::Deflated);
    let mut progress = RescueProgress::new("export", Some(payload_bytes(&files)?), "bytes");
    report(progress.clone());
    for file in &mut files {
        let source_length = file.byte_length;
        zip.start_file(format!("payload/{}", file.path), options)
            .map_err(text)?;
        if let Some(plaintext) = preferences(file)? {
            file.byte_length = plaintext.len() as u64;
            file.sha256 = format!("{:x}", Sha256::digest(&plaintext));
            zip.write_all(&plaintext).map_err(text)?;
            progress.completed += source_length;
            report(progress.clone());
            continue;
        }
        let mut offset = 0;
        let mut hash = Sha256::new();
        while offset < file.byte_length {
            let count = (file.byte_length - offset).min(CHUNK_BYTES as u64) as usize;
            let bytes = read(&file.path, offset, count)?;
            if bytes.len() != count {
                return Err(format!("Source changed or was truncated: {}", file.path));
            }
            hash.update(&bytes);
            zip.write_all(&bytes).map_err(text)?;
            offset += bytes.len() as u64;
            progress.completed += bytes.len() as u64;
            report(progress.clone());
        }
        if !read(&file.path, offset, 1)?.is_empty() {
            return Err(format!("Source grew while exporting: {}", file.path));
        }
        file.sha256 = format!("{:x}", hash.finalize());
    }
    let manifest = RescueManifest {
        format: "operit.data-rescue".into(),
        version: 1,
        layout: layout.into(),
        files,
        metadata,
    };
    let bytes = serde_json::to_vec(&manifest).map_err(text)?;
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err("Manifest exceeds 16 MiB".into());
    }
    let mut finalize = RescueProgress::new("finalizeExport", Some(3), "items");
    report(finalize.clone());
    zip.start_file("manifest.json", options).map_err(text)?;
    zip.write_all(&bytes).map_err(text)?;
    finalize.completed = 1;
    report(finalize.clone());
    zip.finish().map_err(text)?;
    finalize.completed = 2;
    report(finalize);
    Ok(manifest)
}

/// Rejects unsafe or duplicate entries before any restore writes begin.
fn archive_names<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    mut report: impl FnMut(RescueProgress),
) -> Result<BTreeSet<String>, String> {
    if zip.len() == 0 || zip.len() > MAX_FILES + 1 {
        return Err("Invalid archive entry count".into());
    }
    let mut progress = RescueProgress::new("readManifest", Some(zip.len() as u64), "files");
    report(progress.clone());
    let mut names = BTreeSet::new();
    for index in 0..zip.len() {
        let file = zip.by_index_raw(index).map_err(text)?;
        if file.encrypted() {
            return Err("Encrypted rescue archive entries are not supported".into());
        }
        if !names.insert(file.name().to_owned()) {
            return Err("Duplicate archive entry".into());
        }
        if file.is_dir()
            || file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Directory and symlink ZIP entries are not accepted".into());
        }
        progress.completed += 1;
        report(progress.clone());
    }
    Ok(names)
}

/// Verifies exact manifest membership, file lengths, and SHA-256 before publication.
pub fn inspect_archive<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    layout: &str,
    mut report: impl FnMut(RescueProgress),
) -> Result<RescueManifest, String> {
    let names = archive_names(zip, &mut report)?;
    let manifest: RescueManifest = {
        let mut entry = zip.by_name("manifest.json").map_err(text)?;
        if entry.size() > MAX_METADATA_BYTES {
            return Err("Manifest exceeds 16 MiB".into());
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(MAX_METADATA_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(text)?;
        if bytes.len() as u64 > MAX_METADATA_BYTES {
            return Err("Manifest exceeds 16 MiB".into());
        }
        serde_json::from_slice(&bytes).map_err(text)?
    };
    if manifest.format != "operit.data-rescue" || manifest.version != 1 {
        return Err("Unsupported data rescue format or version".into());
    }
    if manifest.layout != layout {
        return Err("This archive uses a different Host storage layout".into());
    }
    validate_files(&manifest.files)?;
    let expected: BTreeSet<_> = std::iter::once("manifest.json".to_string())
        .chain(
            manifest
                .files
                .iter()
                .map(|file| format!("payload/{}", file.path)),
        )
        .collect();
    if names != expected {
        return Err("Archive entries do not match the manifest".into());
    }
    let mut progress = RescueProgress::new(
        "verifyArchive",
        Some(payload_bytes(&manifest.files)?),
        "bytes",
    );
    report(progress.clone());
    for file in &manifest.files {
        read_payload(zip, file, |chunk| {
            progress.completed += chunk.len() as u64;
            report(progress.clone());
            Ok(())
        })?;
    }
    Ok(manifest)
}

/// Streams one file while checking its declared size, digest, and ZIP CRC.
pub fn read_payload<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    file: &RescueFile,
    mut consume: impl FnMut(&[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let mut entry = zip
        .by_name(&format!("payload/{}", file.path))
        .map_err(text)?;
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; CHUNK_BYTES];
    loop {
        let count = entry.read(&mut buffer).map_err(text)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or("Payload size overflow")?;
        if total > file.byte_length {
            return Err(format!("Payload exceeds declared size: {}", file.path));
        }
        hash.update(&buffer[..count]);
        consume(&buffer[..count])?;
    }
    if total != file.byte_length || format!("{:x}", hash.finalize()) != file.sha256 {
        return Err(format!("Payload integrity check failed: {}", file.path));
    }
    Ok(())
}
