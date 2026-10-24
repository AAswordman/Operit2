//! Offline, portable data rescue over Host-owned storage; no Core or database startup.
#![allow(non_snake_case)]

use std::collections::BTreeSet;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use base64::{Engine, engine::general_purpose::STANDARD};
use operit_host_api::ArchiveStagingHost;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zip::{ZipArchive, ZipWriter, CompressionMethod, write::SimpleFileOptions};

pub const CHUNK_BYTES: usize = 64 * 1024;
const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const MAX_FILES: usize = 500_000;

/// Measures completed work within one explicit maintenance stage.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescueProgress {
    pub phase: String,
    pub completed: u64,
    pub total: Option<u64>,
    pub unit: String,
}
impl RescueProgress {
    /// Creates a stage counter whose total is known or still being enumerated.
    pub fn new(phase: &str, total: Option<u64>, unit: &str) -> Self {
        Self { phase: phase.into(), completed: 0, total, unit: unit.into() }
    }
    /// Advances only after the corresponding work has succeeded.
    pub fn advance(&mut self, host: &dyn RescueHost, amount: u64) {
        self.completed += amount;
        host.report_progress(self.clone());
    }
}

/// Adds declared byte lengths without accepting integer overflow.
pub fn payload_bytes(files: &[RescueFile]) -> Result<u64, String> {
    files.iter().try_fold(0u64, |total, file| total.checked_add(file.byte_length).ok_or_else(|| "Payload size overflow".into()))
}

/// Describes one regular file without opening its database or interpreting its contents.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RescueFile {
    pub path: String,
    pub byte_length: u64,
    pub sha256: String,
}

/// Declares every verified payload and the Host layout required to restore it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RescueManifest {
    pub format: String,
    pub version: u32,
    pub layout: String,
    pub files: Vec<RescueFile>,
    pub metadata: Value,
}

/// Owns offline storage, exclusive access, and publication of fully staged imports.
pub trait RescueHost: Send + Sync {
    /// Publishes measured work without waiting for a UI request.
    fn report_progress(&self, progress: RescueProgress);
    /// Identifies the physical storage representation independently of the UI platform.
    fn layout(&self) -> &str;
    /// Describes the selected roots, identities, and any interrupted publication.
    fn describe(&self) -> Result<Value, String>;
    /// Lists every selected file, including bootstrap records and portable secret values.
    fn list_files(&self) -> Result<Vec<RescueFile>, String>;
    /// Reads a bounded range without allocating the entire source file.
    fn read_file(&self, path: &str, offset: u64, length: usize) -> Result<Vec<u8>, String>;
    /// Supplies plaintext preferences for export through the Host storage contract.
    fn export_preferences(&self, path: &str) -> Result<Option<Vec<u8>>, String>;
    /// Creates an isolated destination owned by this import identifier.
    fn begin_restore(&self, id: &str, manifest: &RescueManifest) -> Result<(), String>;
    /// Appends a verified payload chunk to its isolated destination.
    fn write_restore(&self, id: &str, path: &str, chunk: &[u8]) -> Result<(), String>;
    /// Durably completes one staged file, including empty files.
    fn finish_restore_file(&self, id: &str, path: &str) -> Result<(), String>;
    /// Publishes the staged import under a durable maintenance journal.
    fn commit_restore(&self, id: &str, manifest: &RescueManifest) -> Result<Value, String>;
    /// Resumes an explicitly requested interrupted publication from its journal.
    fn resume_restore(&self) -> Result<Value, String>;
    /// Removes only uncommitted files owned by this import identifier.
    fn discard_restore(&self, id: &str) -> Result<(), String>;
}

/// Executes only offline operations and keeps staged archives outside selected data roots.
pub struct DataRescue {
    pub host: Arc<dyn RescueHost>,
    pub archives: Arc<dyn ArchiveStagingHost>,
}

impl DataRescue {
    /// Dispatches a typed operation without ever constructing an application runtime.
    pub fn execute(&self, request: Value) -> Result<Value, String> {
        match string(&request, "operation")? {
            "describe" => self.host.describe(),
            "export" => self.export(),
            "beginUpload" => {
                let id = uuid::Uuid::new_v4().to_string();
                let length = request["byteLength"].as_u64().ok_or("byteLength is required")?;
                if length == 0 { return Err("The selected archive is empty".into()); }
                self.archives.createArchive(&id, length).map_err(text)?;
                Ok(json!({"archiveId": id}))
            }
            "uploadChunk" => {
                let bytes = STANDARD.decode(string(&request, "bytes")?).map_err(text)?;
                if bytes.len() > CHUNK_BYTES { return Err("Upload chunk exceeds 64 KiB".into()); }
                self.archives.appendArchive(archive_id(&request)?, &bytes).map_err(text)?;
                Ok(Value::Null)
            }
            "sealUpload" => Ok(json!({"byteLength": self.archives.sealArchive(archive_id(&request)?).map_err(text)?})),
            "inspect" => Ok(serde_json::to_value(self.inspect(archive_id(&request)?)?).map_err(text)?),
            "restore" => self.restore(archive_id(&request)?),
            "resumeRestore" => self.host.resume_restore(),
            "verifyFiles" => {
                let manifest: RescueManifest = serde_json::from_value(request["manifest"].clone()).map_err(text)?;
                self.verify_storage_manifest(&manifest)?;
                Ok(Value::Null)
            }
            "discardArchive" => {
                self.archives.removeArchive(archive_id(&request)?).map_err(text)?;
                Ok(Value::Null)
            }
            _ => Err("Unknown data rescue operation".into()),
        }
    }

    /// Streams portable payloads and their integrity manifest into Host staging.
    pub fn export(&self) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.archives.createExportArchive(&id).map_err(text)?;
        let result = self.write_export(&id);
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.archives.removeArchive(&id).map_err(|cleanup| format!("{error}; archive cleanup failed: {cleanup}"))?;
                Err(error)
            }
        }
    }

    /// Rechecks persisted staged payloads through the Host before resuming publication.
    pub fn verify_storage_manifest(&self, manifest: &RescueManifest) -> Result<(), String> {
        if manifest.format != "operit.data-rescue" || manifest.version != 1 || manifest.layout != self.host.layout() {
            return Err("Unsupported staged rescue manifest".into());
        }
        validate_files(&manifest.files)?;
        let mut progress = RescueProgress::new("verifyRestored", Some(payload_bytes(&manifest.files)?), "bytes");
        self.host.report_progress(progress.clone());
        for file in &manifest.files {
            let mut offset = 0;
            let mut hash = Sha256::new();
            while offset < file.byte_length {
                let count = (file.byte_length - offset).min(CHUNK_BYTES as u64) as usize;
                let bytes = self.host.read_file(&file.path, offset, count)?;
                if bytes.len() != count { return Err(format!("Staged payload is truncated: {}", file.path)); }
                hash.update(&bytes);
                offset += count as u64;
                progress.advance(self.host.as_ref(), count as u64);
            }
            if !self.host.read_file(&file.path, offset, 1)?.is_empty() || format!("{:x}", hash.finalize()) != file.sha256 {
                return Err(format!("Staged payload integrity check failed: {}", file.path));
            }
        }
        Ok(())
    }

    /// Hashes each exported payload while writing its portable archive entry.
    fn write_export(&self, id: &str) -> Result<Value, String> {
        self.host.report_progress(RescueProgress::new("scan", None, "files"));
        let mut files = self.host.list_files()?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        validate_source_files(&files)?;
        let output = ArchiveIo::new(self.archives.clone(), id, 0);
        let mut zip = ZipWriter::new(output);
        let options = SimpleFileOptions::default().large_file(true)
            .compression_method(CompressionMethod::Deflated);
        // Export progress measures source bytes; plaintext preferences can change output size.
        let mut progress = RescueProgress::new("export", Some(payload_bytes(&files)?), "bytes");
        self.host.report_progress(progress.clone());
        for file in &mut files {
            let source_length = file.byte_length;
            zip.start_file(format!("payload/{}", file.path), options).map_err(text)?;
            if let Some(plaintext) = self.host.export_preferences(&file.path)? {
                file.byte_length = plaintext.len() as u64;
                file.sha256 = format!("{:x}", Sha256::digest(&plaintext));
                zip.write_all(&plaintext).map_err(text)?;
                progress.advance(self.host.as_ref(), source_length);
                continue;
            }
            let mut offset = 0;
            let mut hash = Sha256::new();
            while offset < file.byte_length {
                let count = (file.byte_length - offset).min(CHUNK_BYTES as u64) as usize;
                let bytes = self.host.read_file(&file.path, offset, count)?;
                if bytes.len() != count { return Err(format!("Source changed or was truncated: {}", file.path)); }
                hash.update(&bytes);
                zip.write_all(&bytes).map_err(text)?;
                offset += bytes.len() as u64;
                progress.advance(self.host.as_ref(), bytes.len() as u64);
            }
            if !self.host.read_file(&file.path, offset, 1)?.is_empty() {
                return Err(format!("Source grew while exporting: {}", file.path));
            }
            file.sha256 = format!("{:x}", hash.finalize());
        }
        let mut finalize = RescueProgress::new("finalizeExport", Some(3), "items");
        self.host.report_progress(finalize.clone());
        let manifest = RescueManifest {
            format: "operit.data-rescue".into(), version: 1,
            layout: self.host.layout().into(), files,
            metadata: self.host.describe()?,
        };
        let manifest_bytes = serde_json::to_vec(&manifest).map_err(text)?;
        if manifest_bytes.len() as u64 > MAX_METADATA_BYTES {
            return Err("Manifest exceeds 16 MiB".into());
        }
        zip.start_file("manifest.json", options).map_err(text)?;
        zip.write_all(&manifest_bytes).map_err(text)?;
        finalize.advance(self.host.as_ref(), 1);
        zip.finish().map_err(text)?;
        finalize.advance(self.host.as_ref(), 1);
        let byte_length = self.archives.sealArchive(id).map_err(text)?;
        let reference = self.archives.archiveFileReference(id).map_err(text)?;
        finalize.advance(self.host.as_ref(), 1);
        Ok(json!({"archiveId": id, "byteLength": byte_length, "fileReference": reference, "fileCount": manifest.files.len()}))
    }

    /// Verifies every archive entry and checks exact membership before exposing a preview.
    pub fn inspect(&self, id: &str) -> Result<RescueManifest, String> {
        let length = self.archives.sealArchive(id).map_err(text)?;
        let mut zip = ZipArchive::new(ArchiveIo::new(self.archives.clone(), id, length)).map_err(text)?;
        if zip.len() == 0 || zip.len() > MAX_FILES + 1 { return Err("Invalid archive entry count".into()); }
        let mut scan = RescueProgress::new("readManifest", Some(zip.len() as u64), "files");
        self.host.report_progress(scan.clone());
        let mut names = BTreeSet::new();
        for index in 0..zip.len() {
            let file = zip.by_index_raw(index).map_err(text)?;
            if file.encrypted() { return Err("Encrypted rescue archive entries are not supported".into()); }
            if !names.insert(file.name().to_owned()) { return Err("Duplicate archive entry".into()); }
            if file.is_dir() || file.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) {
                return Err("Directory and symlink ZIP entries are not accepted".into());
            }
            scan.advance(self.host.as_ref(), 1);
        }
        let manifest: RescueManifest = {
            let mut entry = zip.by_name("manifest.json").map_err(text)?;
            if entry.size() > MAX_METADATA_BYTES { return Err("Manifest exceeds 16 MiB".into()); }
            let mut bytes = Vec::new();
            (&mut entry).take(MAX_METADATA_BYTES + 1).read_to_end(&mut bytes).map_err(text)?;
            if bytes.len() as u64 > MAX_METADATA_BYTES { return Err("Manifest exceeds 16 MiB".into()); }
            serde_json::from_slice(&bytes).map_err(text)?
        };
        if manifest.format != "operit.data-rescue" || manifest.version != 1 {
            return Err("Unsupported data rescue format or version".into());
        }
        if manifest.layout != self.host.layout() { return Err("This archive uses a different Host storage layout".into()); }
        validate_files(&manifest.files)?;
        let expected: BTreeSet<_> = std::iter::once("manifest.json".to_string())
            .chain(manifest.files.iter().map(|file| format!("payload/{}", file.path))).collect();
        if names != expected { return Err("Archive entries do not match the manifest".into()); }
        let mut progress = RescueProgress::new("verifyArchive", Some(payload_bytes(&manifest.files)?), "bytes");
        self.host.report_progress(progress.clone());
        for file in &manifest.files {
            let mut entry = zip.by_name(&format!("payload/{}", file.path)).map_err(text)?;
            verify_payload(&mut entry, file, |chunk| { progress.advance(self.host.as_ref(), chunk.len() as u64); Ok(()) })?;
        }
        Ok(manifest)
    }

    /// Verifies the entire archive, stages complete files, then publishes one explicit restore.
    pub fn restore(&self, id: &str) -> Result<Value, String> {
        let manifest = self.inspect(id)?;
        let restore_id = uuid::Uuid::new_v4().to_string();
        if let Err(error) = self.host.begin_restore(&restore_id, &manifest) {
            let cleanup = self.host.discard_restore(&restore_id);
            return match cleanup {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!("{error}; staging cleanup failed: {cleanup_error}")),
            };
        }
        let result = self.stage_restore(id, &restore_id, &manifest);
        if let Err(error) = result {
            self.host.discard_restore(&restore_id).map_err(|cleanup| format!("{error}; staging cleanup failed: {cleanup}"))?;
            return Err(error);
        }
        self.host.commit_restore(&restore_id, &manifest)
    }

    /// Rechecks each payload while streaming it into isolated Host destinations.
    fn stage_restore(&self, id: &str, restore_id: &str, manifest: &RescueManifest) -> Result<(), String> {
        let length = self.archives.sealArchive(id).map_err(text)?;
        let mut zip = ZipArchive::new(ArchiveIo::new(self.archives.clone(), id, length)).map_err(text)?;
        let mut progress = RescueProgress::new("restore", Some(payload_bytes(&manifest.files)?), "bytes");
        self.host.report_progress(progress.clone());
        for file in &manifest.files {
            let mut entry = zip.by_name(&format!("payload/{}", file.path)).map_err(text)?;
            verify_payload(&mut entry, file, |chunk| {
                self.host.write_restore(restore_id, &file.path, chunk)?;
                progress.advance(self.host.as_ref(), chunk.len() as u64);
                Ok(())
            })?;
            self.host.finish_restore_file(restore_id, &file.path)?;
        }
        Ok(())
    }
}

/// Validates one payload path before it reaches any Host filesystem operation.
pub fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.len() > 4096 || path.contains('\\') || path.contains(':') || path.contains('\0') ||
        path.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
        return Err(format!("Invalid rescue path: {path}"));
    }
    Ok(())
}

/// Rejects duplicate, excessive, or malformed manifest records.
fn validate_files(files: &[RescueFile]) -> Result<(), String> {
    if files.len() > MAX_FILES { return Err("Archive exceeds the file count limit".into()); }
    let mut paths = BTreeSet::new();
    for file in files {
        validate_path(&file.path)?;
        if file.sha256.len() != 64 || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("Invalid SHA-256 digest: {}", file.path));
        }
        if !paths.insert(file.path.as_str()) { return Err(format!("Duplicate rescue path: {}", file.path)); }
    }
    validate_ancestors(&paths)?;
    Ok(())
}

/// Validates source paths before their authenticated digests are computed.
fn validate_source_files(files: &[RescueFile]) -> Result<(), String> {
    if files.len() > MAX_FILES { return Err("Archive exceeds the file count limit".into()); }
    let mut paths = BTreeSet::new();
    for file in files {
        validate_path(&file.path)?;
        if !paths.insert(file.path.as_str()) { return Err(format!("Duplicate rescue path: {}", file.path)); }
    }
    validate_ancestors(&paths)?;
    Ok(())
}

/// Rejects file ancestors even when sibling names separate them in lexical order.
fn validate_ancestors(paths: &BTreeSet<&str>) -> Result<(), String> {
    for path in paths {
        for (index, _) in path.match_indices('/') {
            if paths.contains(&path[..index]) {
                return Err(format!("Rescue paths overlap: {} and {}", &path[..index], path));
            }
        }
    }
    Ok(())
}

/// Verifies bounded decompression, declared size, SHA-256, and ZIP authentication.
fn verify_payload(reader: &mut impl Read, file: &RescueFile, mut consume: impl FnMut(&[u8]) -> Result<(), String>) -> Result<(), String> {
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; CHUNK_BYTES];
    loop {
        let count = reader.read(&mut buffer).map_err(text)?;
        if count == 0 { break; }
        total = total.checked_add(count as u64).ok_or("Payload size overflow")?;
        if total > file.byte_length { return Err(format!("Payload exceeds declared size: {}", file.path)); }
        hash.update(&buffer[..count]);
        consume(&buffer[..count])?;
    }
    if total != file.byte_length || format!("{:x}", hash.finalize()) != file.sha256 {
        return Err(format!("Payload integrity check failed: {}", file.path));
    }
    Ok(())
}

/// Extracts an exact string field from a structured command.
fn string<'a>(request: &'a Value, field: &str) -> Result<&'a str, String> {
    request[field].as_str().ok_or_else(|| format!("Missing string field: {field}"))
}

/// Accepts only UUID identifiers issued by the archive service.
fn archive_id(request: &Value) -> Result<&str, String> {
    let id = string(request, "archiveId")?;
    uuid::Uuid::parse_str(id).map_err(text)?;
    Ok(id)
}

/// Converts a concrete error without hiding its original diagnostic text.
fn text(error: impl std::fmt::Display) -> String { error.to_string() }

/// Adapts Host-owned bounded staging to the standard ZIP seekable I/O contract.
struct ArchiveIo { host: Arc<dyn ArchiveStagingHost>, id: String, position: u64, length: u64 }
impl ArchiveIo {
    /// Creates an independent cursor over one staged archive.
    fn new(host: Arc<dyn ArchiveStagingHost>, id: &str, length: u64) -> Self {
        Self { host, id: id.into(), position: 0, length }
    }
}
impl Read for ArchiveIo {
    /// Reads at most one bounded Host range and advances this cursor.
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = buffer.len().min(CHUNK_BYTES).min(self.length.saturating_sub(self.position) as usize);
        if count == 0 { return Ok(0); }
        let bytes = self.host.readArchive(&self.id, self.position, count).map_err(io::Error::other)?;
        if bytes.len() != count { return Err(io::Error::other("Truncated staged archive")); }
        buffer[..count].copy_from_slice(&bytes);
        self.position += count as u64;
        Ok(count)
    }
}
impl Write for ArchiveIo {
    /// Writes ZIP payloads and header updates in bounded Host chunks.
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        for chunk in buffer.chunks(CHUNK_BYTES) {
            self.host.writeExportArchive(&self.id, self.position, chunk).map_err(io::Error::other)?;
            self.position += chunk.len() as u64;
            self.length = self.length.max(self.position);
        }
        Ok(buffer.len())
    }
    /// Leaves durability to the Host archive sealing operation.
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
impl Seek for ArchiveIo {
    /// Rejects cursor overflow and negative ZIP seeks explicitly.
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::Current(delta) => i128::from(self.position) + i128::from(delta),
            SeekFrom::End(delta) => i128::from(self.length) + i128::from(delta),
        };
        self.position = u64::try_from(position).map_err(io::Error::other)?;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rejects traversal, platform separators, and empty path segments.
    #[test]
    fn rescue_paths_reject_boundary_escapes() {
        for path in ["", "runtime/../secret", "runtime\\file", "runtime//file", "runtime/:file"] {
            assert!(validate_path(path).is_err(), "accepted invalid path: {path}");
        }
        assert!(validate_path("runtime/identities/a/data.bin").is_ok());
    }

    /// Rejects malformed digests and namespace prefix collisions in manifests.
    #[test]
    fn manifests_require_exact_authenticated_files() {
        let valid = RescueFile { path: "runtime/file".into(), byte_length: 0, sha256: "0".repeat(64) };
        assert!(validate_files(std::slice::from_ref(&valid)).is_ok());
        let invalid_digest = RescueFile { sha256: "x".repeat(64), ..valid.clone() };
        assert!(validate_files(std::slice::from_ref(&invalid_digest)).is_err());
        let collision = [
            valid,
            RescueFile { path: "runtime/file-sibling".into(), byte_length: 0, sha256: "0".repeat(64) },
            RescueFile { path: "runtime/file/child".into(), byte_length: 0, sha256: "0".repeat(64) },
        ];
        assert!(validate_files(&collision).is_err());
    }
}
