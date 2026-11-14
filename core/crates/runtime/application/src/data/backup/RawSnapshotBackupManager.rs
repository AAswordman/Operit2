use std::io::{Seek, Write};
use std::sync::Arc;

use operit_data_rescue::secrets::{PortableSecrets, SYSTEM_SECRETS};
use operit_data_rescue::{archive, secrets, RescueFile, RescueManifest};

use operit_host_api::{HostSecretStore, RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_store::PreferencesExport::{
    exportPreferences, isPreferencesPath, readPreferencesExportBytes, readPreferencesExportKey,
};
use operit_util::RuntimeStorageLayout;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::data::archive::ArchiveSource::{ArchiveSource, ArchiveSourceReader};

const SNAPSHOT_LAYOUT: &str = "operit.runtime.v1";

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
        let secretBytes = serde_json::to_vec(&secrets::read_secrets(self.secretStore.as_deref())?)
            .map_err(|error| error.to_string())?;
        let mut files = self.collectFileEntries(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH)?;
        files.push(RescueFile {
            path: SYSTEM_SECRETS.into(),
            byte_length: secretBytes.len() as u64,
            sha256: String::new(),
        });
        archive::write_archive(
            output,
            SNAPSHOT_LAYOUT,
            serde_json::json!({"createdAt": operit_host_api::TimeUtils::currentTimeMillis()}),
            files,
            |path, offset, length| {
                if path == SYSTEM_SECRETS {
                    let start = usize::try_from(offset)
                        .map_err(|error| error.to_string())?
                        .min(secretBytes.len());
                    let end = start.saturating_add(length).min(secretBytes.len());
                    return Ok(secretBytes[start..end].to_vec());
                }
                self.storageHost
                    .readBytesRange(path, offset, length)
                    .map_err(|error| error.to_string())
            },
            |file| self.exportPreferences(&file.path, file.byte_length),
            |_| {},
        )?;
        Ok(())
    }

    /// Replaces runtime storage contents from a range-readable snapshot archive source.
    pub(crate) fn restoreSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<(), String> {
        let (mut zip, manifest) = self.openSnapshot(source)?;
        let secretFile = manifest
            .files
            .iter()
            .find(|file| file.path == SYSTEM_SECRETS)
            .ok_or("Snapshot is missing its Host secret payload")?;
        if secretFile.byte_length > 16 * 1024 * 1024 {
            return Err("Host secret payload exceeds 16 MiB".into());
        }
        let mut bytes = Vec::new();
        archive::read_payload(&mut zip, secretFile, |chunk| {
            bytes.extend_from_slice(chunk);
            Ok(())
        })?;
        let imported: PortableSecrets =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        secrets::validate_destination(&imported, self.secretStore.as_deref())?;
        self.storageHost
            .delete(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH, true)
            .map_err(|error| error.to_string())?;
        for file in manifest
            .files
            .iter()
            .filter(|file| file.path != SYSTEM_SECRETS)
        {
            let mut writer = self
                .storageWriteHost
                .createWriteSession(&file.path)
                .map_err(|error| error.to_string())?;
            let result = archive::read_payload(&mut zip, file, |chunk| {
                writer.writeChunk(chunk).map_err(|error| error.to_string())
            });
            if let Err(error) = result {
                writer.discard().map_err(|cleanup| {
                    format!("{error}; failed to discard incomplete runtime storage file: {cleanup}")
                })?;
                return Err(error);
            }
            writer.commit().map_err(|error| error.to_string())?;
        }
        secrets::restore_secrets(&imported, self.secretStore.as_deref())
    }

    /// Reads snapshot manifest metadata from a range-readable source without writing storage.
    pub(crate) fn inspectSnapshotSource(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<RawSnapshotManifest, String> {
        let (_, manifest) = self.openSnapshot(source)?;
        Ok(RawSnapshotManifest {
            formatVersion: manifest.version as i32,
            createdAt: manifest
                .metadata
                .get("createdAt")
                .and_then(serde_json::Value::as_i64)
                .ok_or("Snapshot is missing createdAt")?,
            includes: manifest
                .files
                .into_iter()
                .filter(|file| file.path != SYSTEM_SECRETS)
                .map(|file| file.path)
                .collect(),
        })
    }

    /// Uses the shared archive verifier and enforces the runtime snapshot's storage scope.
    fn openSnapshot(
        &self,
        source: Arc<dyn ArchiveSource>,
    ) -> Result<(ZipArchive<ArchiveSourceReader>, RescueManifest), String> {
        let mut zip =
            ZipArchive::new(ArchiveSourceReader::new(source)).map_err(|error| error.to_string())?;
        let manifest = archive::inspect_archive(&mut zip, SNAPSHOT_LAYOUT, |_| {})?;
        if !manifest
            .files
            .iter()
            .any(|file| file.path == SYSTEM_SECRETS)
        {
            return Err("Snapshot is missing its Host secret payload".into());
        }
        for file in &manifest.files {
            if file.path != SYSTEM_SECRETS && !file.path.starts_with("runtime/") {
                return Err(format!("invalid snapshot runtime path: {}", file.path));
            }
        }
        Ok((zip, manifest))
    }

    /// Lists runtime payloads for the shared archive writer to sort and hash.
    #[allow(non_snake_case)]
    fn collectFileEntries(&self, prefix: &str) -> Result<Vec<RescueFile>, String> {
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
                files.push(RescueFile {
                    path: entry.path,
                    byte_length: byteLength,
                    sha256: String::new(),
                });
            }
        }
        Ok(files)
    }

    /// Uses the common plaintext preference contract without changing the source storage.
    fn exportPreferences(&self, path: &str, byteLength: u64) -> Result<Option<Vec<u8>>, String> {
        if isPreferencesPath(path) {
            let bytes = readPreferencesExportBytes(self.storageHost.as_ref(), path, byteLength)?;
            let plaintext = exportPreferences(path, bytes, || {
                readPreferencesExportKey(self.secretStore.as_deref())
            })?;
            return Ok(Some(plaintext));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use operit_host_native_storage::{NativeFileSecretStore, NativeRuntimeStorageHost};
    use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
    use operit_store::PreferencesEncryption::PreferencesEncryption;
    use operit_store::SyncOperationStore::{SyncClock, SyncOperationStore};
    use serde_json::json;
    use std::io::{Cursor, Read};
    use zip::{write::SimpleFileOptions, ZipWriter};

    struct BytesSource(Vec<u8>);
    impl ArchiveSource for BytesSource {
        /// Returns the immutable fixture archive length.
        fn len(&self) -> Result<u64, String> {
            Ok(self.0.len() as u64)
        }
        /// Reads only the exact requested fixture range.
        fn readAt(&self, offset: u64, length: usize) -> Result<Vec<u8>, String> {
            self.0
                .get(offset as usize..offset as usize + length)
                .map(<[u8]>::to_vec)
                .ok_or("Invalid fixture range".into())
        }
    }

    struct Fixture {
        root: std::path::PathBuf,
        host: Arc<NativeRuntimeStorageHost>,
        secrets: Arc<NativeFileSecretStore>,
    }
    impl Fixture {
        /// Creates isolated source or newly installed destination storage.
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("operit-snapshot-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            let root = std::fs::canonicalize(root).unwrap();
            Self {
                host: Arc::new(NativeRuntimeStorageHost::new(
                    root.join("runtime"),
                    root.join("workspaces"),
                )),
                secrets: Arc::new(NativeFileSecretStore(root.join("secrets"))),
                root,
            }
        }
        /// Uses the production snapshot manager over the isolated Host implementations.
        fn manager(&self) -> RawSnapshotBackupManager {
            RawSnapshotBackupManager::new(
                self.host.clone(),
                self.host.clone(),
                Some(self.secrets.clone()),
            )
        }
        /// Exports a complete snapshot through the shared production archive writer.
        fn export(&self) -> Vec<u8> {
            let mut output = Cursor::new(Vec::new());
            self.manager().exportSnapshot(&mut output).unwrap();
            output.into_inner()
        }
    }
    impl Drop for Fixture {
        /// Removes only the UUID-owned fixture directory after database calls have completed.
        fn drop(&mut self) {
            assert_eq!(
                self.root.parent(),
                Some(
                    std::fs::canonicalize(std::env::temp_dir())
                        .unwrap()
                        .as_path()
                )
            );
            assert!(self
                .root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("operit-snapshot-test-"));
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    /// Builds a registered preferences key with deterministic fixture material.
    fn key() -> Vec<u8> {
        serde_json::to_vec(&json!({"format":"operit.preferences.encryption.key","version":1,"algorithm":"XChaCha20Poly1305","keyId":"snapshot-test-key","key":base64::engine::general_purpose::STANDARD_NO_PAD.encode([9u8;32])})).unwrap()
    }

    /// Restores a fresh installation, reads encrypted journal history, and writes another entry.
    #[test]
    fn snapshot_fresh_install_restores_key_and_encrypted_history() {
        let source = Fixture::new();
        let key = key();
        source
            .secrets
            .writeSecret(secrets::SECRET_KEYS[0], &key)
            .unwrap();
        operit_store::setDefaultHostSecretStore(source.secrets.clone());
        let path = RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH;
        let store = PreferencesDataStore::newEncryptedWithStorage(source.host.clone(), path);
        store
            .edit(|values| values.set(&stringPreferencesKey("api_key"), "portable-snapshot-token"))
            .unwrap();
        let ciphertext = source.host.readBytes(path).unwrap();
        let plaintext = PreferencesEncryption::decodeKeyBytes(&key)
            .unwrap()
            .decrypt(path, &ciphertext)
            .unwrap();
        source
            .host
            .writeBytes("runtime/data/large.bin", &vec![23; 140_000])
            .unwrap();
        source.host.writeBytes("runtime/data/empty", b"").unwrap();
        let expected = SyncOperationStore::new(source.host.clone(), "runtime/sync")
            .operationsSince(&SyncClock::empty(), &[], 100)
            .unwrap();
        assert!(!expected.is_empty());
        let bytes = source.export();
        let mut zip = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let portable = archive::inspect_archive(&mut zip, SNAPSHOT_LAYOUT, |_| {}).unwrap();
        assert!(portable
            .files
            .iter()
            .any(|file| file.path == SYSTEM_SECRETS));
        let mut actual = Vec::new();
        zip.by_name(&format!("payload/{path}"))
            .unwrap()
            .read_to_end(&mut actual)
            .unwrap();
        assert_eq!(actual, plaintext);
        assert_eq!(source.host.readBytes(path).unwrap(), ciphertext);
        source
            .secrets
            .deleteSecret(secrets::SECRET_KEYS[0])
            .unwrap();
        let destination = Fixture::new();
        operit_store::setDefaultHostSecretStore(destination.secrets.clone());
        assert!(PreferencesEncryption::load_existing().is_err());
        let archiveSource = Arc::new(BytesSource(bytes));
        let preview = destination
            .manager()
            .inspectSnapshotSource(archiveSource.clone())
            .unwrap();
        assert_eq!(preview.formatVersion, 1);
        destination
            .manager()
            .restoreSnapshotSource(archiveSource)
            .unwrap();
        assert_eq!(
            destination
                .secrets
                .readSecret(secrets::SECRET_KEYS[0])
                .unwrap(),
            Some(key)
        );
        assert_eq!(
            destination
                .host
                .readBytes("runtime/data/large.bin")
                .unwrap(),
            vec![23; 140_000]
        );
        assert_eq!(
            destination.host.readBytes("runtime/data/empty").unwrap(),
            b""
        );
        let journal = SyncOperationStore::new(destination.host.clone(), "runtime/sync");
        assert_eq!(
            journal
                .operationsSince(&SyncClock::empty(), &[], 100)
                .unwrap(),
            expected
        );
        let restored =
            PreferencesDataStore::newEncryptedWithStorage(destination.host.clone(), path);
        restored
            .edit(|values| values.set(&stringPreferencesKey("provider_new"), "new-provider"))
            .unwrap();
        assert_eq!(
            restored
                .data()
                .unwrap()
                .get(&stringPreferencesKey("provider_new"))
                .unwrap(),
            &json!("new-provider")
        );
        assert!(
            journal
                .operationsSince(&SyncClock::empty(), &[], 100)
                .unwrap()
                .len()
                > expected.len()
        );
    }

    /// Rewrites a ZIP payload with a valid CRC but keeps its original SHA-256 manifest.
    fn change_payload(bytes: Vec<u8>, target: &str, replacement: &[u8]) -> Vec<u8> {
        let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut output = ZipWriter::new(Cursor::new(Vec::new()));
        for index in 0..input.len() {
            let mut file = input.by_index(index).unwrap();
            output
                .start_file(file.name(), SimpleFileOptions::default())
                .unwrap();
            if file.name() == target {
                output.write_all(replacement).unwrap();
            } else {
                std::io::copy(&mut file, &mut output).unwrap();
            }
        }
        output.finish().unwrap().into_inner()
    }

    /// Fails before deleting destination files or replacing secrets when hashes disagree.
    #[test]
    fn snapshot_corrupt_payload_preserves_destination() {
        let source = Fixture::new();
        source
            .host
            .writeBytes("runtime/data/file", b"original")
            .unwrap();
        let bytes = change_payload(source.export(), "payload/runtime/data/file", b"tampered");
        let destination = Fixture::new();
        destination
            .host
            .writeBytes("runtime/data/keep", b"keep")
            .unwrap();
        destination
            .secrets
            .writeSecret(secrets::SECRET_KEYS[0], b"existing-key")
            .unwrap();
        let error = destination
            .manager()
            .restoreSnapshotSource(Arc::new(BytesSource(bytes)))
            .unwrap_err();
        assert!(error.contains("integrity check failed"), "{error}");
        assert_eq!(
            destination.host.readBytes("runtime/data/keep").unwrap(),
            b"keep"
        );
        assert_eq!(
            destination
                .secrets
                .readSecret(secrets::SECRET_KEYS[0])
                .unwrap()
                .unwrap(),
            b"existing-key"
        );
    }

    /// Rejects destinations without secret storage before replacing any existing files.
    #[test]
    fn snapshot_requires_destination_secret_capability() {
        let source = Fixture::new();
        source
            .secrets
            .writeSecret(secrets::SECRET_KEYS[0], &key())
            .unwrap();
        let destination = Fixture::new();
        destination
            .host
            .writeBytes("runtime/data/keep", b"keep")
            .unwrap();
        let manager =
            RawSnapshotBackupManager::new(destination.host.clone(), destination.host.clone(), None);
        assert!(manager
            .restoreSnapshotSource(Arc::new(BytesSource(source.export())))
            .unwrap_err()
            .contains("does not support"));
        assert_eq!(
            destination.host.readBytes("runtime/data/keep").unwrap(),
            b"keep"
        );
    }

    /// Replaces stale destination keys with the archive's exact portable secret set.
    #[test]
    fn snapshot_secret_restore_matches_data_rescue_replacement() {
        let source = Fixture::new();
        source
            .secrets
            .writeSecret(secrets::SECRET_KEYS[0], &key())
            .unwrap();
        let destination = Fixture::new();
        destination
            .secrets
            .writeSecret(secrets::SECRET_KEYS[0], b"old-key")
            .unwrap();
        destination
            .secrets
            .writeSecret(secrets::SECRET_KEYS[1], b"stale-oauth")
            .unwrap();
        destination
            .manager()
            .restoreSnapshotSource(Arc::new(BytesSource(source.export())))
            .unwrap();
        assert_eq!(
            destination
                .secrets
                .readSecret(secrets::SECRET_KEYS[0])
                .unwrap(),
            Some(key())
        );
        assert!(destination
            .secrets
            .readSecret(secrets::SECRET_KEYS[1])
            .unwrap()
            .is_none());
    }

    /// Rejects the removed raw snapshot format without changing destination files or keys.
    #[test]
    fn snapshot_rejects_legacy_format_before_restore() {
        let destination = Fixture::new();
        let path = RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH;
        destination
            .host
            .writeBytes("runtime/data/keep", b"keep")
            .unwrap();
        destination
            .secrets
            .writeSecret(secrets::SECRET_KEYS[0], b"existing-key")
            .unwrap();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        serde_json::to_writer(
            &mut zip,
            &json!({"formatVersion":1,"createdAt":1,"includes":[path]}),
        )
        .unwrap();
        zip.start_file(format!("payload/{path}"), SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"provider":"legacy"}"#).unwrap();
        let source = Arc::new(BytesSource(zip.finish().unwrap().into_inner()));
        assert!(destination
            .manager()
            .inspectSnapshotSource(source.clone())
            .is_err());
        assert!(destination.manager().restoreSnapshotSource(source).is_err());
        assert_eq!(
            destination.host.readBytes("runtime/data/keep").unwrap(),
            b"keep"
        );
        assert_eq!(
            destination
                .secrets
                .readSecret(secrets::SECRET_KEYS[0])
                .unwrap()
                .unwrap(),
            b"existing-key"
        );
    }
}
