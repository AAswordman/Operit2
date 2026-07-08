use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const FORMAT_VERSION: i32 = 1;
const ENTRY_MANIFEST: &str = "manifest.json";
const ENTRY_PAYLOAD_PREFIX: &str = "payload/";

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
}

impl RawSnapshotBackupManager {
    /// Creates a snapshot manager over the supplied runtime storage host.
    pub fn new(storageHost: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storageHost }
    }

    /// Serializes the full runtime storage tree into a ZIP snapshot.
    #[allow(non_snake_case)]
    pub fn exportSnapshot(&self) -> Result<Vec<u8>, String> {
        let files = self.collectFiles(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH)?;
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
            for (path, content) in files {
                zip.start_file(format!("{ENTRY_PAYLOAD_PREFIX}{path}"), options)
                    .map_err(|error| error.to_string())?;
                zip.write_all(&content).map_err(|error| error.to_string())?;
            }
            zip.finish().map_err(|error| error.to_string())?;
        }
        Ok(out.into_inner())
    }

    /// Replaces runtime storage contents with the payload from a snapshot archive.
    #[allow(non_snake_case)]
    pub fn restoreSnapshot(&self, bytes: Vec<u8>) -> Result<(), String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
        let manifest = readManifest(&mut archive)?;
        if manifest.formatVersion != FORMAT_VERSION {
            return Err(format!(
                "unsupported snapshot formatVersion: {}",
                manifest.formatVersion
            ));
        }
        let mut payloadFiles = Vec::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
            let name = file.name().to_string();
            if name == ENTRY_MANIFEST || file.is_dir() {
                continue;
            }
            let Some(path) = name.strip_prefix(ENTRY_PAYLOAD_PREFIX) else {
                return Err(format!("invalid snapshot entry: {name}"));
            };
            validateSnapshotPath(path)?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)
                .map_err(|error| error.to_string())?;
            payloadFiles.push((path.to_string(), content));
        }
        self.storageHost
            .delete(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH, true)
            .map_err(|error| error.to_string())?;
        for (path, content) in payloadFiles {
            self.storageHost
                .writeBytes(&path, &content)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Reads snapshot manifest metadata without writing to storage.
    #[allow(non_snake_case)]
    pub fn inspectSnapshot(&self, bytes: Vec<u8>) -> Result<RawSnapshotManifest, String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
        readManifest(&mut archive)
    }

    /// Collects sorted file paths and bytes from a storage subtree.
    #[allow(non_snake_case)]
    fn collectFiles(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
        let mut files = Vec::new();
        for entry in self
            .storageHost
            .list(prefix)
            .map_err(|error| error.to_string())?
        {
            if entry.isDirectory {
                files.extend(self.collectFiles(&entry.path)?);
            } else {
                files.push((
                    entry.path.clone(),
                    self.storageHost
                        .readBytes(&entry.path)
                        .map_err(|error| error.to_string())?,
                ));
            }
        }
        files.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(files)
    }
}

/// Reads and decodes the manifest entry from a snapshot archive.
#[allow(non_snake_case)]
fn readManifest(archive: &mut ZipArchive<Cursor<Vec<u8>>>) -> Result<RawSnapshotManifest, String> {
    let mut manifestFile = archive
        .by_name(ENTRY_MANIFEST)
        .map_err(|error| error.to_string())?;
    let mut manifestText = String::new();
    manifestFile
        .read_to_string(&mut manifestText)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&manifestText).map_err(|error| error.to_string())
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
