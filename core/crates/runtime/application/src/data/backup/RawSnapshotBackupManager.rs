use std::collections::BTreeMap;
use std::io::{Seek, Write};
use std::sync::Arc;

use operit_data_rescue::secrets::{PortableSecrets, SYSTEM_SECRETS};
use operit_data_rescue::{archive, secrets, RescueFile, RescueManifest};

use operit_host_api::{HostSecretStore, RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_store::PreferencesExport::{
    exportPreferences, isPreferencesPath, readPreferencesExportBytes, readPreferencesExportKey,
};
use operit_store::SyncOperationStore::SyncOperationStore;
use operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore;
use operit_util::RuntimeStorageLayout;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::data::archive::ArchiveSource::{ArchiveSource, ArchiveSourceReader};

const SNAPSHOT_LAYOUT: &str = "operit.runtime.v1";
const EXPORT_STAGING_ROOT: &str = "runtime/temp/clean_on_exit/raw_snapshot_export";
const LIVE_ONLY_PATHS: [&str; 4] = [
    "runtime/logs",
    "runtime/sync.rollback",
    "runtime/sync/transactions.sqlite-shm",
    EXPORT_STAGING_ROOT,
];

/// Owns only this export's private copies, including on an early failure.
struct SnapshotStaging<'a> {
    host: &'a dyn RuntimeStorageHost,
    root: String,
    files: BTreeMap<String, Option<(String, u64)>>,
}

impl SnapshotStaging<'_> {
    fn capture(&mut self, writers: &dyn RuntimeStorageWriteHost, path: &str) -> Result<(), String> {
        if !self.host.exists(path).map_err(|error| error.to_string())? {
            self.files.insert(path.into(), None);
            return Ok(());
        }
        let target = format!("{}/{}", self.root, self.files.len());
        let mut writer = writers
            .createWriteSession(&target)
            .map_err(|error| error.to_string())?;
        let result = (|| {
            let mut offset = 0;
            loop {
                let bytes = self
                    .host
                    .readBytesRange(path, offset, operit_data_rescue::CHUNK_BYTES)
                    .map_err(|error| error.to_string())?;
                if bytes.is_empty() {
                    return Ok(offset);
                }
                writer
                    .writeChunk(&bytes)
                    .map_err(|error| error.to_string())?;
                offset += bytes.len() as u64;
            }
        })();
        let length = match result {
            Ok(length) => length,
            Err(error) => {
                let _ = writer.discard();
                return Err(error);
            }
        };
        writer.commitFast().map_err(|error| error.to_string())?;
        self.files.insert(path.into(), Some((target, length)));
        Ok(())
    }
}

impl Drop for SnapshotStaging<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.host.delete(&self.root, true) {
            operit_host_api::logHostError("SnapshotExport", &error.to_string());
        }
    }
}

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
        let mut staging = SnapshotStaging {
            host: self.storageHost.as_ref(),
            root: format!("{EXPORT_STAGING_ROOT}/{}", uuid::Uuid::new_v4()),
            files: BTreeMap::new(),
        };
        let mut capture = |path: &str| staging.capture(self.storageWriteHost.as_ref(), path);
        WorkspaceFileSyncStore::new(self.storageHost.clone(), RUNTIME_SYNC_DIR_PATH)
            .captureSnapshotInventory(&mut capture)?;
        SyncOperationStore::new(self.storageHost.clone(), RUNTIME_SYNC_DIR_PATH)
            .captureSnapshotFiles(capture)?;
        let mut files =
            self.collectFileEntries(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH, &staging)?;
        files.extend(staging.files.iter().filter_map(|(path, staged)| {
            let (_, length) = staged.as_ref()?;
            Some(RescueFile {
                path: path.clone(),
                byte_length: *length,
                sha256: String::new(),
            })
        }));
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
                let source = staging
                    .files
                    .get(path)
                    .and_then(Option::as_ref)
                    .map_or(path, |(staged, _)| staged.as_str());
                self.storageHost
                    .readBytesRange(source, offset, length)
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
        self.clearSnapshotDirectory(RuntimeStorageLayout::RUNTIME_ROOT_DIR_PATH)?;
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
        let mut manifest = archive::inspect_archive(&mut zip, SNAPSHOT_LAYOUT, |_| {})?;
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
        manifest.files.retain(|file| !self.storageHost.isSnapshotExcludedPath(&file.path));
        Ok((zip, manifest))
    }

    /// Lists runtime payloads for the shared archive writer to sort and hash.
    #[allow(non_snake_case)]
    fn collectFileEntries(
        &self,
        prefix: &str,
        staging: &SnapshotStaging<'_>,
    ) -> Result<Vec<RescueFile>, String> {
        let mut files = Vec::new();
        for entry in self
            .storageHost
            .list(prefix)
            .map_err(|error| error.to_string())?
        {
            if self.storageHost.isSnapshotExcludedPath(&entry.path)
                || staging.files.contains_key(&entry.path)
                || LIVE_ONLY_PATHS.iter().any(|path| *path == entry.path)
            {
                continue;
            }
            if entry.isDirectory {
                files.extend(self.collectFileEntries(&entry.path, staging)?);
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

    /// Removes replaced application data while retaining Host-owned entries and their ancestors.
    fn clearSnapshotDirectory(&self, prefix: &str) -> Result<bool, String> {
        let mut retained = false;
        for entry in self.storageHost.list(prefix).map_err(|error| error.to_string())? {
            if self.storageHost.isSnapshotExcludedPath(&entry.path)
                || (entry.isDirectory && self.clearSnapshotDirectory(&entry.path)?)
            {
                retained = true;
                continue;
            }
            self.storageHost.delete(&entry.path, true).map_err(|error| error.to_string())?;
        }
        Ok(retained)
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
    use operit_store::SyncOperationStore::SyncClock;
    use serde_json::json;
    use std::io::{Cursor, Read, SeekFrom};
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

    #[test]
    fn snapshot_restores_copied_markers_with_or_without_existing_sync_state() {
        for includeSync in [false, true] {
            let source = Fixture::new();
            let output = "runtime/data/chat_outputs/chat";
            let policy = format!("{output}/.operit/sync.json");
            let database =
                format!("{output}/identities/old/runtime/plugin_data/space/demo/example.sqlite");
            let marker = format!("{database}.operit-storage.json");
            let markerBytes = br#"{"path":"runtime/plugin_data/space/demo/example.sqlite","owner":"demo","kind":"sqlite","sync":true}"#;
            source.host.writeBytes(&database, b"copied file").unwrap();
            source.host.writeBytes(&marker, markerBytes).unwrap();
            assert!(!source.host.exists(&policy).unwrap());
            if includeSync {
                assert_eq!(WorkspaceFileSyncStore::new(source.host.clone(), RUNTIME_SYNC_DIR_PATH)
                    .scan().unwrap(), 2);
            }
            let destination = Fixture::new();
            destination.manager()
                .restoreSnapshotSource(Arc::new(BytesSource(source.export())))
                .unwrap();
            assert_eq!(destination.host.exists(RUNTIME_SYNC_DIR_PATH).unwrap(), includeSync);
            assert_eq!(destination.host.readBytes(&database).unwrap(), b"copied file");
            assert_eq!(destination.host.readBytes(&marker).unwrap(), markerBytes);

            let workspace =
                WorkspaceFileSyncStore::new(destination.host.clone(), RUNTIME_SYNC_DIR_PATH);
            // Archive restore assigns new file times; re-inventory once, then stay idle.
            assert_eq!(workspace.scan().unwrap(), 2);
            let journal = SyncOperationStore::new(destination.host.clone(), RUNTIME_SYNC_DIR_PATH);
            let operations = journal.operationsSince(&SyncClock::empty(), &[], 100).unwrap();
            let versions = destination.host.readBytes("runtime/sync/entity_versions.jsonl").unwrap();
            for _ in 0..5 {
                assert_eq!(WorkspaceFileSyncStore::new(destination.host.clone(), RUNTIME_SYNC_DIR_PATH)
                    .scan().unwrap(), 0);
            }
            assert_eq!(journal.operationsSince(&SyncClock::empty(), &[], 100).unwrap(), operations);
            assert_eq!(destination.host.readBytes("runtime/sync/entity_versions.jsonl").unwrap(), versions);

            // Subsequent application writes and another device restore use the same path.
            workspace.trackPaths(&[&database], || {
                destination.host.writeBytes(&database, b"edited copied file")
                    .map_err(|error| error.to_string())
            }).unwrap();
            let next = Fixture::new();
            next.manager()
                .restoreSnapshotSource(Arc::new(BytesSource(destination.export())))
                .unwrap();
            assert_eq!(next.host.readBytes(&database).unwrap(), b"edited copied file");
            assert_eq!(next.host.readBytes(&marker).unwrap(), markerBytes);
            assert!(!next.host.exists(&policy).unwrap());
            let restoredWorkspace = WorkspaceFileSyncStore::new(next.host.clone(), RUNTIME_SYNC_DIR_PATH);
            assert_eq!(restoredWorkspace.scan().unwrap(), 2);
            assert_eq!(restoredWorkspace.scan().unwrap(), 0);
        }
    }

    /// Exports portable data and preserves destination Host environments during replacement.
    #[test]
    fn snapshot_excludes_host_storage_and_preserves_destination_environment() {
        let source = Fixture::new();
        for path in [
            "runtime/terminal/rootfs/image/usr/lib/systemd/system/file",
            "runtime/identities/a/terminal/rootfs/image/sdcard/file",
            "runtime/ohos-vroot/rootfs/bin/sh",
            "runtime/config/preferences/terminal_profiles.json",
        ] {
            source.host.writeBytes(path, b"source Host bytes").unwrap();
        }
        #[cfg(unix)]
        {
            let rootfs = source.root.join("runtime/terminal/rootfs/image");
            std::fs::write(rootfs.join("systemd\\x2dcryptsetup.slice"), b"nonportable filename").unwrap();
            std::os::unix::fs::symlink("/missing-guest-mount", rootfs.join("sdcard")).unwrap();
        }
        for path in [
            "runtime/data/record", "runtime/terminal/rootfs-notes/record",
            "runtime/data/terminal/rootfs/record",
        ] {
            source.host.writeBytes(path, b"portable data").unwrap();
        }
        let bytes = source.export();
        let manifest = source.manager().inspectSnapshotSource(Arc::new(BytesSource(bytes.clone()))).unwrap();
        assert_eq!(manifest.includes.len(), 3);
        assert!(manifest.includes.iter().all(|path| !source.host.isSnapshotExcludedPath(path)));

        let target = Fixture::new();
        let private = [
            "runtime/terminal/rootfs/local/sdcard/record",
            "runtime/identities/local/terminal/rootfs/local/bin/sh",
            "runtime/ohos-vroot/rootfs/bin/sh",
            "runtime/config/preferences/terminal_profiles.json",
        ];
        for path in private { target.host.writeBytes(path, b"target Host bytes").unwrap(); }
        target.host.writeBytes("runtime/data/stale", b"stale data").unwrap();
        target.manager().restoreSnapshotSource(Arc::new(BytesSource(bytes))).unwrap();
        assert!(!target.host.exists("runtime/data/stale").unwrap());
        assert!(!target.host.exists("runtime/terminal/rootfs/image").unwrap());
        assert_eq!(target.host.readBytes("runtime/data/record").unwrap(), b"portable data");
        for path in private { assert_eq!(target.host.readBytes(path).unwrap(), b"target Host bytes"); }
    }

    /// Restores old snapshots while ignoring source-device environments and launch profiles.
    #[test]
    fn snapshot_import_ignores_archived_host_storage() {
        let payloads = [
            ("runtime/data/record", b"portable data".as_slice()),
            ("runtime/terminal/rootfs/source/bin/sh", b"source environment".as_slice()),
            ("runtime/config/preferences/terminal_profiles.json", b"source profiles".as_slice()),
            (SYSTEM_SECRETS, b"{}".as_slice()),
        ];
        let mut output = Cursor::new(Vec::new());
        archive::write_archive(&mut output, SNAPSHOT_LAYOUT, json!({"createdAt":1}),
            payloads.iter().map(|(path, bytes)| RescueFile {
                path: (*path).into(), byte_length: bytes.len() as u64, sha256: String::new(),
            }).collect(),
            |path, offset, length| {
                let bytes = payloads.iter().find(|(name, _)| *name == path).unwrap().1;
                let start = (offset as usize).min(bytes.len());
                Ok(bytes[start..start.saturating_add(length).min(bytes.len())].to_vec())
            }, |_| Ok(None), |_| {}).unwrap();
        let target = Fixture::new();
        let profiles = "runtime/config/preferences/terminal_profiles.json";
        target.host.writeBytes(profiles, b"target profiles").unwrap();
        let source = Arc::new(BytesSource(output.into_inner()));
        assert_eq!(target.manager().inspectSnapshotSource(source.clone()).unwrap().includes, vec!["runtime/data/record"]);
        target.manager().restoreSnapshotSource(source).unwrap();
        assert_eq!(target.host.readBytes("runtime/data/record").unwrap(), b"portable data");
        assert_eq!(target.host.readBytes(profiles).unwrap(), b"target profiles");
        assert!(!target.host.exists("runtime/terminal/rootfs/source").unwrap());
    }

    /// Keeps live logs out of exports and captures sync state before compression starts.
    #[test]
    fn snapshot_excludes_live_logs_and_captures_sync_files_before_compression() {
        struct MutatingOutput<'a> {
            output: Cursor<Vec<u8>>,
            mutate: Option<Box<dyn FnOnce() + 'a>>,
        }
        impl Write for MutatingOutput<'_> {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if let Some(mutate) = self.mutate.take() {
                    mutate();
                }
                self.output.write(bytes)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.output.flush()
            }
        }
        impl Seek for MutatingOutput<'_> {
            fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
                self.output.seek(position)
            }
        }

        let source = Fixture::new();
        let log = "runtime/logs/operit.log";
        source.host.writeBytes(log, b"before").unwrap();
        source
            .host
            .writeBytes("runtime/logs/operit.log.manual-test.bak", b"old log")
            .unwrap();
        source
            .host
            .writeBytes("runtime/sync.rollback/old.sqlite", b"old journal")
            .unwrap();
        source
            .host
            .writeBytes("runtime/logs-extra/keep", b"user data")
            .unwrap();
        let workspace = WorkspaceFileSyncStore::new(source.host.clone(), RUNTIME_SYNC_DIR_PATH);
        let journal = SyncOperationStore::new(source.host.clone(), RUNTIME_SYNC_DIR_PATH);
        let versions = "runtime/sync/entity_versions.jsonl";
        source
            .host
            .writeBytes("workspaces/test/before", b"user file")
            .unwrap();
        assert_eq!(workspace.scan().unwrap(), 1);
        let capturedVersions = source.host.readBytes(versions).unwrap();
        let expected = journal
            .operationsSince(&SyncClock::empty(), &[], 100)
            .unwrap();

        let mut output = MutatingOutput {
            output: Cursor::new(Vec::new()),
            mutate: Some(Box::new(|| {
                source.host.appendBytes(log, b"after").unwrap();
                source
                    .host
                    .writeBytes("workspaces/test/while-exporting", b"second user file")
                    .unwrap();
                assert_eq!(workspace.scan().unwrap(), 1);
                assert!(source.host.readBytes(versions).unwrap().len() > capturedVersions.len());
            })),
        };
        source.manager().exportSnapshot(&mut output).unwrap();
        assert!(source.host.list(EXPORT_STAGING_ROOT).unwrap().is_empty());
        let archive = Arc::new(BytesSource(output.output.into_inner()));
        let destination = Fixture::new();
        destination
            .manager()
            .restoreSnapshotSource(archive)
            .unwrap();
        for path in ["runtime/logs", "runtime/sync.rollback", EXPORT_STAGING_ROOT] {
            assert!(
                !destination.host.exists(path).unwrap(),
                "unexpected restore: {path}"
            );
        }
        assert_eq!(
            destination
                .host
                .readBytes("runtime/logs-extra/keep")
                .unwrap(),
            b"user data"
        );
        assert_eq!(
            destination.host.readBytes(versions).unwrap(),
            capturedVersions
        );
        let restored = SyncOperationStore::new(destination.host.clone(), RUNTIME_SYNC_DIR_PATH);
        assert_eq!(
            restored
                .operationsSince(&SyncClock::empty(), &[], 100)
                .unwrap(),
            expected
        );
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
