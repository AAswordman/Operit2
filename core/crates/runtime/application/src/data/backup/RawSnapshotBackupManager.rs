use std::io::{Read, Seek, Write};
use std::sync::Arc;

use operit_host_api::{
    HostSecretStore, RuntimeStorageHost, RuntimeStorageWriteHost, RuntimeStorageWriteSession,
};
use operit_store::PreferencesExport::{
    exportPreferences, isPreferencesPath, readPreferencesExportBytes, readPreferencesExportKey,
};
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
    secretStore: Option<Arc<dyn HostSecretStore>>,
}

impl RawSnapshotBackupManager {
    /// Creates a snapshot manager over the supplied runtime storage host.
    pub fn new(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
        secretStore: Option<Arc<dyn HostSecretStore>>,
    ) -> Self {
        Self {
            storageHost,
            storageWriteHost,
            secretStore,
        }
    }

    /// Writes a ZIP snapshot directly into a host-backed seekable output.
    #[allow(non_snake_case)]
    pub(crate) fn exportSnapshot<W: Write + Seek>(&self, output: W) -> Result<(), String> {
        let files = self.collectFileEntries(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH)?;
        let manifest = RawSnapshotManifest {
            formatVersion: FORMAT_VERSION,
            createdAt: currentTimeMillis(),
            includes: files.iter().map(|(path, _)| path.clone()).collect(),
        };
        let mut zip = ZipWriter::new(output);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .large_file(true);
        zip.start_file(ENTRY_MANIFEST, options)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(&mut zip, &manifest).map_err(|error| error.to_string())?;
        for (path, byteLength) in files {
            zip.start_file(format!("{ENTRY_PAYLOAD_PREFIX}{path}"), options)
                .map_err(|error| error.to_string())?;
            self.copyStorageFileToZip(&path, byteLength, &mut zip)?;
        }
        zip.finish().map_err(|error| error.to_string())?;
        Ok(())
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
    fn copyStorageFileToZip<W: Write + Seek>(
        &self,
        path: &str,
        byteLength: u64,
        zip: &mut ZipWriter<W>,
    ) -> Result<(), String> {
        if isPreferencesPath(path) {
            let bytes = readPreferencesExportBytes(self.storageHost.as_ref(), path, byteLength)?;
            if bytes.len() as u64 != byteLength {
                return Err(format!("Preferences changed while exporting: {path}"));
            }
            let plaintext = exportPreferences(path, bytes, || {
                readPreferencesExportKey(self.storageHost.as_ref(), self.secretStore.as_deref())
            })?;
            zip.write_all(&plaintext)
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
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
            if chunk.len() > requestLength {
                return Err(format!(
                    "runtime storage range exceeds the requested length: {path}"
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use operit_store::PreferencesEncryption::PreferencesEncryption;
    use serde_json::json;
    use std::io::Cursor;

    /// Verifies plaintext snapshot entries without changing the encrypted source preference file.
    #[test]
    fn snapshot_preferences_are_portable_plaintext() {
        let root = std::env::temp_dir().join(format!(
            "operit-snapshot-preferences-{}",
            uuid::Uuid::new_v4()
        ));
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        let key = serde_json::to_vec(&json!({
            "format": "operit.preferences.encryption.key", "version": 1,
            "algorithm": "XChaCha20Poly1305", "keyId": "snapshot-test-key",
            "key": base64::engine::general_purpose::STANDARD_NO_PAD.encode([9u8; 32]),
        }))
        .unwrap();
        let path = RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH;
        let plaintext = br#"{"api_key":"portable-snapshot-token"}"#;
        let encrypted = PreferencesEncryption::decodeKeyBytes(&key)
            .unwrap()
            .encrypt(path, plaintext)
            .unwrap();
        host.writeBytes(RuntimeStorageLayout::PREFERENCES_ENCRYPTION_KEY_PATH, &key)
            .unwrap();
        host.writeBytes(path, &encrypted).unwrap();
        let manager = RawSnapshotBackupManager::new(host.clone(), host.clone(), None);
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(format!("payload/{path}"), SimpleFileOptions::default())
            .unwrap();
        manager
            .copyStorageFileToZip(path, encrypted.len() as u64, &mut zip)
            .unwrap();
        let output = zip.finish().unwrap();
        let mut archive = ZipArchive::new(output).unwrap();
        let mut actual = Vec::new();
        let mut entry = archive.by_name(&format!("payload/{path}")).unwrap();
        assert!(!entry.encrypted());
        entry.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, plaintext);
        assert_eq!(host.readBytes(path).unwrap(), encrypted);
        let resolved = std::fs::canonicalize(&root).unwrap();
        assert_eq!(
            resolved.parent(),
            Some(
                std::fs::canonicalize(std::env::temp_dir())
                    .unwrap()
                    .as_path()
            )
        );
        assert!(resolved
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("operit-snapshot-preferences-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
}
