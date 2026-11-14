use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::{NativeFileSecretStore, NativeRuntimeStorageHost};
use sha2::{Digest, Sha256};

use crate::data::archive::ArchiveSource::{ArchiveSource, ArchiveSourceReader};
use super::RawSnapshotBackupManager::RawSnapshotBackupManager;

/// Reads a supplied archive without copying its entire contents into memory.
struct DiagnosticArchiveSource(Mutex<File>);

/// Reproduces credential selection for the supplied gateway without restoring storage or sending HTTP.
#[test]
#[ignore]
fn diagnose_external_raw_snapshot_empty_gateway_key_pool() {
    use operit_model::ModelConfigData::ProviderProfile;
    use operit_providers::chat::llmprovider::ApiKeyProvider::ApiKeyPoolState;
    let input = std::env::var("RAW_SNAPSHOT_INPUT").expect("RAW_SNAPSHOT_INPUT is required");
    let mut zip = zip::ZipArchive::new(File::open(input).unwrap()).unwrap();
    let preferences: serde_json::Value = serde_json::from_reader(zip.by_name(
        "payload/runtime/config/preferences/model_configs.preferences.json",
    ).unwrap()).unwrap();
    let value = preferences["provider_DEEPSEEK"].clone();
    let provider: ProviderProfile = match value.as_str() {
        Some(raw) => serde_json::from_str(raw).unwrap(),
        None => serde_json::from_value(value).unwrap(),
    };
    assert!(provider.useMultipleApiKeys);
    assert!(provider.apiKeyPool.is_empty());
    assert!(!provider.apiKey.trim().is_empty());
    let result = ApiKeyPoolState::default().reserve(&mut provider.clone(), 0);
    assert!(matches!(&result, Err(error) if error.starts_with("API key pool has no usable keys")));
    eprintln!("Confirmed: the shared credential getter rejects the empty enabled pool before HTTP.");
}

impl ArchiveSource for DiagnosticArchiveSource {
    /// Returns the read-only source file length.
    fn len(&self) -> Result<u64, String> {
        self.0.lock().unwrap().metadata().map(|value| value.len()).map_err(|error| error.to_string())
    }

    /// Reads one bounded range from the read-only source handle.
    fn readAt(&self, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start(offset)).map_err(|error| error.to_string())?;
        let mut bytes = vec![0; length];
        file.read_exact(&mut bytes).map_err(|error| error.to_string())?;
        Ok(bytes)
    }
}

/// Restores an external snapshot exclusively into a newly allocated diagnostic Host root.
#[test]
#[ignore]
fn diagnose_external_raw_snapshot_in_isolated_storage() {
    use operit_store::RuntimeStorageHost::{
        setDefaultHostSecretStore, setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost,
    };
    let input = std::env::var("RAW_SNAPSHOT_INPUT").expect("RAW_SNAPSHOT_INPUT is required");
    let parent = std::env::var("RAW_SNAPSHOT_DIAGNOSTIC_ROOT").expect("RAW_SNAPSHOT_DIAGNOSTIC_ROOT is required");
    let parent = std::fs::canonicalize(parent).expect("existing diagnostic parent directory");
    let root = parent.join(format!("raw-snapshot-diagnostic-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    assert_eq!(root.parent(), Some(parent.as_path()));
    let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
    let secrets = Arc::new(NativeFileSecretStore(root.join("secrets")));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host.clone());
    setDefaultHostSecretStore(secrets.clone());
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
        operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
    ));
    let source: Arc<dyn ArchiveSource> = Arc::new(DiagnosticArchiveSource(Mutex::new(File::open(input).unwrap())));
    let manager = RawSnapshotBackupManager::new(host.clone(), host.clone(), Some(secrets));
    eprintln!("Diagnostic storage: {}", root.display());
    let manifest = manager.inspectSnapshotSource(source.clone()).expect("snapshot inspection");
    eprintln!("Snapshot format={} files={}", manifest.formatVersion, manifest.includes.len());
    manager.restoreSnapshotSource(source.clone()).expect("isolated snapshot restore");

    let mut zip = zip::ZipArchive::new(ArchiveSourceReader::new(source)).unwrap();
    let mut count = 0;
    let mut total_bytes = 0;
    let mut databases = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).unwrap();
        let Some(path) = entry.name().strip_prefix("payload/").map(str::to_string) else { continue };
        if entry.is_dir() || !path.starts_with("runtime/") { continue; }
        let mut expected = Sha256::new();
        let mut actual = Sha256::new();
        let mut offset = 0;
        let mut buffer = [0; 64 * 1024];
        loop {
            let length = entry.read(&mut buffer).unwrap();
            if length == 0 { break; }
            expected.update(&buffer[..length]);
            let bytes = host.readBytesRange(&path, offset, length).unwrap();
            assert_eq!(bytes.len(), length, "restored size mismatch: {path}");
            actual.update(bytes);
            offset += length as u64;
        }
        assert_eq!(expected.finalize(), actual.finalize(), "restored hash mismatch: {path}");
        assert!(host.readBytesRange(&path, offset, 1).unwrap().is_empty());
        if path.ends_with(".sqlite") { databases.push(path); }
        count += 1;
        total_bytes += offset;
    }
    assert_eq!(count, manifest.includes.len());
    eprintln!("Verified {count} files, {total_bytes} bytes against source archive");

    for path in databases {
        let connection = rusqlite::Connection::open_with_flags(
            root.join(&path), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ).unwrap();
        let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0)).unwrap();
        assert_eq!(integrity, "ok", "database integrity: {path}");
        eprintln!("Database integrity OK: {path}");
    }
    let database = operit_store::db::AppDatabase::AppDatabase::default().expect("open restored Core database");
    for table in ["chats", "messages", "message_parts"] {
        let count: i64 = database.store().queryScalar(&format!("SELECT COUNT(*) FROM {table}"), Vec::new()).unwrap();
        eprintln!("Core {table}={count}");
    }
    let providers = crate::data::preferences::ModelConfigManager::ModelConfigManager::default()
        .getProviderProfiles().expect("read restored model configurations");
    let bindings = crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager::default()
        .functionModelBindingFlow().unwrap().first().expect("read restored function bindings");
    eprintln!("Model providers={} function bindings={}", providers.len(), bindings.len());
    let writer = super::CharacterPluginMigration::CharacterPluginWriter::open(host.clone(), host.clone())
        .expect("open restored character plugin");
    for collection in ["cards", "stores", "owners", "memories", "links"] {
        eprintln!("Plugin {collection}={}", writer.list(collection).expect("read plugin collection").len());
    }
    operit_store::db::AppDatabase::AppDatabase::closeDatabase();
}
