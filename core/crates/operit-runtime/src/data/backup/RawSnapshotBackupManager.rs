use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use operit_host_api::{RuntimeStorageHost, RuntimeStorageWriteHost, RuntimeStorageWriteSession};
use operit_util::RuntimeStorageLayout;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::data::archive::ArchiveSource::{ArchiveSource, ArchiveSourceReader};

const FORMAT_VERSION: i32 = 1;
const ENTRY_MANIFEST: &str = "manifest.json";
const ENTRY_PAYLOAD_PREFIX: &str = "payload/";
const STORAGE_WRITE_CHUNK_BYTES: usize = 64 * 1024;

/// Describes the contents and format version of a raw runtime snapshot archive.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct RawSnapshotManifest {
    pub formatVersion: i32,
    pub createdAt: i64,
    pub includes: Vec<String>,
}

/// Exports, restores, and inspects raw runtime storage snapshots.
#[derive(Clone)]
pub struct RawSnapshotBackupManager {
    storageHost: Arc<dyn RuntimeStorageHost>,
    storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
}

impl RawSnapshotBackupManager {
    /// Creates a snapshot manager over the supplied runtime storage host.
    pub fn new(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
    ) -> Self {
        Self {
            storageHost,
            storageWriteHost,
        }
    }

    /// Serializes the full runtime storage tree into a ZIP snapshot.
    #[allow(non_snake_case)]
    pub fn exportSnapshot(&self) -> Result<Vec<u8>, String> {
        let files = self.collectFileEntries(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH)?;
        let manifest = RawSnapshotManifest {
            formatVersion: FORMAT_VERSION,
            createdAt: currentTimeMillis(),
            includes: files.iter().map(|(path, _)| path.clone()).collect(),
        };
        let mut out = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut out);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file(ENTRY_MANIFEST, options)
                .map_err(|error| error.to_string())?;
            zip.write_all(
                serde_json::to_string_pretty(&manifest)
                    .map_err(|error| error.to_string())?
                    .as_bytes(),
            )
            .map_err(|error| error.to_string())?;
            for (path, byteLength) in files {
                zip.start_file(format!("{ENTRY_PAYLOAD_PREFIX}{path}"), options)
                    .map_err(|error| error.to_string())?;
                self.copyStorageFileToZip(&path, byteLength, &mut zip)?;
            }
            zip.finish().map_err(|error| error.to_string())?;
        }
        Ok(out.into_inner())
    }

    /// Replaces runtime storage contents from a range-readable snapshot archive source.
    pub(crate) fn restoreSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<(), String> {
        let mut archive = openSnapshotArchive(source.clone())?;
        let manifest = readManifest(&mut archive)?;
        if manifest.formatVersion != FORMAT_VERSION {
            return Err(format!(
                "unsupported snapshot formatVersion: {}",
                manifest.formatVersion
            ));
        }
        let mut payloadEntries = Vec::new();
        for index in 0..archive.len() {
            let file = archive.by_index(index).map_err(|error| error.to_string())?;
            let name = file.name().to_string();
            if name == ENTRY_MANIFEST || file.is_dir() {
                continue;
            }
            let Some(path) = name.strip_prefix(ENTRY_PAYLOAD_PREFIX) else {
                return Err(format!("invalid snapshot entry: {name}"));
            };
            validateSnapshotPath(path)?;
            let storagePath = path.to_string();
            payloadEntries.push((name, storagePath));
        }
        for (entryName, _) in &payloadEntries {
            let mut file = archive
                .by_name(entryName)
                .map_err(|error| error.to_string())?;
            std::io::copy(&mut file, &mut std::io::sink()).map_err(|error| error.to_string())?;
        }
        self.storageHost
            .delete(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH, true)
            .map_err(|error| error.to_string())?;
        let mut archive = openSnapshotArchive(source)?;
        for (entryName, path) in payloadEntries {
            let mut file = archive
                .by_name(&entryName)
                .map_err(|error| error.to_string())?;
            let mut writer = self
                .storageWriteHost
                .createWriteSession(&path)
                .map_err(|error| error.to_string())?;
            if let Err(error) = copySnapshotEntryToStorage(&mut file, writer.as_mut()) {
                let discardResult = writer
                    .discard()
                    .map_err(|discardError| discardError.to_string());
                return match discardResult {
                    Ok(()) => Err(error),
                    Err(discardError) => Err(format!(
                        "{error}; failed to discard incomplete runtime storage file: {discardError}"
                    )),
                };
            }
            writer.commit().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Reads snapshot manifest metadata from a range-readable source without writing storage.
    pub(crate) fn inspectSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<RawSnapshotManifest, String> {
        let mut archive = openSnapshotArchive(source)?;
        readManifest(&mut archive)
    }

    /// Collects sorted file paths and immutable lengths from a storage subtree.
    #[allow(non_snake_case)]
    fn collectFileEntries(&self, prefix: &str) -> Result<Vec<(String, u64)>, String> {
        let mut files = Vec::new();
        for entry in self
            .storageHost
            .list(prefix)
            .map_err(|error| error.to_string())?
        {
            if entry.isDirectory {
                files.extend(self.collectFileEntries(&entry.path)?);
            } else {
                let byteLength = u64::try_from(entry.size).map_err(|_| {
                    format!("runtime storage file has invalid size: {}", entry.path)
                })?;
                files.push((entry.path.clone(), byteLength));
            }
        }
        files.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(files)
    }

    /// Copies one immutable runtime storage file into the current ZIP entry in bounded chunks.
    #[allow(non_snake_case)]
    fn copyStorageFileToZip(
        &self,
        path: &str,
        byteLength: u64,
        zip: &mut ZipWriter<&mut Cursor<Vec<u8>>>,
    ) -> Result<(), String> {
        let mut offset = 0u64;
        while offset < byteLength {
            let remaining = byteLength - offset;
            let requestLength = usize::try_from(remaining.min(STORAGE_WRITE_CHUNK_BYTES as u64))
                .map_err(|_| {
                    "runtime storage range length does not fit this platform".to_string()
                })?;
            let chunk = self
                .storageHost
                .readBytesRange(path, offset, requestLength)
                .map_err(|error| error.to_string())?;
            if chunk.is_empty() {
                return Err(format!(
                    "runtime storage file ended before its declared length: {path}"
                ));
            }
            zip.write_all(&chunk).map_err(|error| error.to_string())?;
            offset = offset
                .checked_add(
                    u64::try_from(chunk.len())
                        .map_err(|_| "runtime storage chunk length does not fit u64".to_string())?,
                )
                .ok_or_else(|| "runtime storage offset overflowed".to_string())?;
        }
        Ok(())
    }
}

/// Streams one ZIP payload entry into a host-owned runtime storage write session.
fn copySnapshotEntryToStorage(
    source: &mut impl Read,
    writer: &mut dyn RuntimeStorageWriteSession,
) -> Result<(), String> {
    let mut buffer = [0u8; STORAGE_WRITE_CHUNK_BYTES];
    loop {
        let count = source
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(());
        }
        writer
            .writeChunk(&buffer[..count])
            .map_err(|error| error.to_string())?;
    }
}

/// Reads and decodes the manifest entry from a snapshot archive.
#[allow(non_snake_case)]
fn readManifest(
    archive: &mut ZipArchive<impl Read + std::io::Seek>,
) -> Result<RawSnapshotManifest, String> {
    let mut manifestFile = archive
        .by_name(ENTRY_MANIFEST)
        .map_err(|error| error.to_string())?;
    let mut manifestText = String::new();
    manifestFile
        .read_to_string(&mut manifestText)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&manifestText).map_err(|error| error.to_string())
}

/// Opens a ZIP archive over one immutable range-readable snapshot source.
fn openSnapshotArchive(
    source: Arc<dyn ArchiveSource>,
) -> Result<ZipArchive<ArchiveSourceReader>, String> {
    ZipArchive::new(ArchiveSourceReader::new(source)).map_err(|error| error.to_string())
}

/// Validates that an archive payload entry is a relative storage path.
#[allow(non_snake_case)]
fn validateSnapshotPath(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains('\\')
        || path.split('/').any(|segment| {
            segment.is_empty() || segment == "." || segment == ".." || segment.contains(':')
        })
    {
        return Err(format!("invalid snapshot path: {path}"));
    }
    Ok(())
}

/// Returns the host-provided current time in milliseconds.
#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
