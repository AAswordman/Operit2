use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{NativeRuntimeStorageHost, PosixFileSystemHost};
use operit_runtime::services::FileSelectionService::FileSelectionService;
use operit_runtime::services::ChatBackupDirectoryService::ChatBackupDirectoryService;
use operit_vfs::VfsPath;
use operit_vfs::VisualFileSystem::VisualFileSystem;
use operit_util::stream::ReverseStream::ReverseStream;

/// Owns an isolated host filesystem used by the picker and the VFS reader.
struct Fixture {
    root: PathBuf,
    service: FileSelectionService,
    vfs: VisualFileSystem,
    backup: ChatBackupDirectoryService,
}

impl Fixture {
    /// Connects the picker and VFS to exactly the same runtime storage host.
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "operit-file-selection-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        let runtime = root.join("runtime");
        let workspaces = root.join("workspaces");
        let storage = Arc::new(NativeRuntimeStorageHost::new(
            runtime.clone(),
            workspaces.clone(),
        ));
        let files = Arc::new(PosixFileSystemHost::new());
        let host = HostManager {
            runtimeStorageHost: Some(storage.clone()),
            runtimeStorageWriteHost: Some(storage.clone()),
            fileSystemHost: Some(files.clone()),
            ..HostManager::default()
        };
        Self {
            root,
            service: FileSelectionService::getInstance(&host).unwrap(),
            backup: ChatBackupDirectoryService::getInstance(&host).unwrap(),
            vfs: VisualFileSystem::fromStorage(files, storage).unwrap(),
        }
    }
}

/// Detects deleted destinations instead of recreating them, and preserves unrelated files.
#[test]
fn backup_directory_publication_and_missing_directory_checks() {
    let fixture = Fixture::new();
    let storage = Arc::new(NativeRuntimeStorageHost::new(fixture.root.join("runtime"), fixture.root.join("workspaces")));
    operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost(storage.clone());
    operit_store::RuntimeStorageHost::setDefaultRuntimeSqliteHost(storage);
    let selected = fixture.root.join("user-backups");
    std::fs::create_dir(&selected).unwrap();
    std::fs::write(selected.join("unrelated.txt"), "keep").unwrap();
    let source = serde_json::json!({
        "namespace":"/virtual/local/folders", "backend":"native", "root":selected, "name":"Backups",
    }).to_string();
    let directory = fixture.service.mountDirectory(source).unwrap();
    fixture.backup.checkDirectory(directory.clone()).unwrap();
    fixture.backup.checkDirectory(directory.clone()).unwrap();
    assert_eq!(std::fs::read_dir(&selected).unwrap().count(), 1);
    assert_eq!(std::fs::read_to_string(selected.join("unrelated.txt")).unwrap(), "keep");
    assert!(fixture.backup.checkDirectory(selected.to_string_lossy().into_owned()).is_err());
    let name = "operit-chats-source-2026-11-19-123.sqlite";
    std::fs::write(selected.join(name), b"incomplete database").unwrap();
    assert!(fixture.backup.createBackup(directory.clone(), name.into()).is_err());
    let database = operit_store::db::AppDatabase::AppDatabase::default().unwrap();
    database.chatDao().insertChat(operit_model::ChatEntity::ChatEntity::new("backup-chat".into(),"Snapshot".into(),1)).unwrap();
    let valid = "operit-chats-source-2026-11-19-456.sqlite";
    let info = fixture.backup.createBackup(directory.clone(),valid.into()).unwrap();
    assert_eq!(info.createdAt, 456);
    assert_eq!(fixture.backup.listBackups(directory.clone()).unwrap().len(), 3);
    assert!(std::fs::read(selected.join(valid)).unwrap().starts_with(b"SQLite format 3\0"));
    database.store().execute("UPDATE chats SET title='After backup'", vec![]).unwrap();
    assert!(fixture.backup.restoreBackup(directory.clone(), "operit-chats-missing-789.sqlite".into()).is_err());
    assert_eq!(database.store().queryScalar::<String>("SELECT title FROM chats", vec![]).unwrap(), "After backup");
    let lease = database.store().beginChatExecution("backup-chat").unwrap();
    assert!(fixture.backup.restoreBackup(directory.clone(), valid.into()).is_err());
    lease.release().unwrap();
    fixture.backup.restoreBackup(directory.clone(),valid.into()).unwrap();
    assert_eq!(database.store().queryScalar::<String>("SELECT title FROM chats", vec![]).unwrap(), "Snapshot");
    assert!(operit_store::sync::SqlChatSyncStore::SqlChatSyncStore::default().unwrap().restoration().unwrap().is_some());
    assert_eq!(database.store().queryScalar::<i64>("SELECT count(*) FROM _operit_transaction_initialization", vec![]).unwrap(), 0);
    assert!(fixture.backup.removeBackup(directory.clone(), "unrelated.txt".into()).is_err());
    assert!(fixture.backup.createBackup(directory.clone(), "operit-chats-../escape.sqlite".into()).is_err());
    std::fs::rename(&selected, fixture.root.join("moved-backups")).unwrap();
    assert!(fixture.backup.checkDirectory(directory).is_err());
    assert!(!selected.exists());
    assert_eq!(std::fs::read_to_string(fixture.root.join("moved-backups/unrelated.txt")).unwrap(), "keep");
    operit_store::db::AppDatabase::AppDatabase::closeDatabase();
}

impl Drop for Fixture {
    /// Removes only this fixture's verified directory under the OS temporary root.
    fn drop(&mut self) {
        assert!(self.root.is_absolute());
        assert_eq!(self.root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

/// Reads imported JSON through the same VFS surface used by file tools.
#[tokio::test]
async fn selected_json_is_readable_through_vfs_after_stream_commit() {
    let fixture = Fixture::new();
    let content = br#"{"entries":{"0":{"content":"world book","key":["world"]}}}"#;
    let path = fixture
        .service
        .createFile("Reborn2.4.2.json".into())
        .unwrap();
    let (mut sender, stream) = ReverseStream::channel();
    let (sent, written) = tokio::join!(
        async {
            for chunk in content.chunks(7) {
                sender.send(chunk.to_vec()).await?;
            }
            sender.close();
            Ok::<_, String>(())
        },
        fixture
            .service
            .writeFile(path.clone(), content.len() as i64, stream),
    );
    sent.unwrap();
    written.unwrap();
    assert!(path.starts_with("/operitapp/data/temp/clean_on_exit/file_picker/"));
    assert!(path.ends_with("/Reborn2.4.2.json"));
    assert_eq!(fixture.vfs.readFile(&path).unwrap().as_bytes(), content);
    assert_eq!(
        fixture.vfs.fileExists(&path).unwrap().size,
        content.len() as i64
    );
    fixture.service.deleteFile(path.clone()).unwrap();
    assert!(!fixture.vfs.fileExists(&path).unwrap().exists);
}

/// Rejects truncated streams without exposing a partially written VFS file.
#[tokio::test]
async fn truncated_selection_is_not_published() {
    let fixture = Fixture::new();
    let path = fixture.service.createFile("book.json".into()).unwrap();
    let (mut sender, stream) = ReverseStream::channel();
    sender.send(b"{}".to_vec()).await.unwrap();
    sender.close();
    let error = fixture
        .service
        .writeFile(path.clone(), 3, stream)
        .await
        .unwrap_err();
    assert!(error.starts_with("Selected file byte length mismatch"));
    assert!(!fixture.vfs.fileExists(&path).unwrap().exists);
}

/// Prevents physical source paths from becoming destinations for host writes.
#[tokio::test]
async fn physical_paths_and_oversized_chunks_are_rejected() {
    let fixture = Fixture::new();
    let (mut sender, stream) = ReverseStream::channel();
    sender.close();
    assert!(fixture
        .service
        .writeFile("/private/var/mobile/tmp/book.json".into(), 0, stream,)
        .await
        .is_err());

    let path = fixture.service.createFile("book.json".into()).unwrap();
    let (mut sender, stream) = ReverseStream::channel();
    sender.send(vec![0; 1024 * 1024 + 1]).await.unwrap();
    sender.close();
    assert!(fixture
        .service
        .writeFile(path.clone(), 1024 * 1024 + 1, stream)
        .await
        .is_err());
    assert!(!fixture.vfs.fileExists(&path).unwrap().exists);
}

/// Resolves selected directories through their registered host mount backend.
#[test]
fn directory_selection_returns_a_readable_vfs_mount() {
    let fixture = Fixture::new();
    let selected = fixture.root.join("selected");
    std::fs::create_dir(&selected).unwrap();
    std::fs::write(selected.join("book.json"), b"{}").unwrap();
    let source = serde_json::json!({
        "namespace": "/virtual/local/folders",
        "backend": "native",
        "root": selected,
        "name": "selected",
    });
    let path = fixture.service.mountDirectory(source.to_string()).unwrap();
    assert!(path.starts_with("/virtual/local/folders/"));
    assert!(fixture.vfs.fileExists(&path).unwrap().isDirectory);
    assert_eq!(
        fixture.vfs.readFile(&format!("{path}/book.json")).unwrap(),
        "{}"
    );
}
