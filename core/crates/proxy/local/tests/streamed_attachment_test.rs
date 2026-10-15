use operit_host_api::{HostError, HostResult, RuntimeStorageWriteHost, RuntimeStorageWriteSession};
use std::io::{Read, Seek, SeekFrom};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::{HostManager, setDefaultHostRuntimeTaskSchedulerHost};
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{CoreLinkSharedClient, CorePushRequest, toCoreValue};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::services::AttachmentTransferManager::AttachmentTransferManager;
use operit_util::stream::ReverseStream::ReverseStream;
use serde_json::json;

const CHUNK: usize = 1024 * 1024;

struct FailingWriteHost(Arc<AtomicUsize>);
struct FailingSession(Arc<AtomicUsize>);

impl RuntimeStorageWriteHost for FailingWriteHost {
    fn createWriteSession(&self, path: &str) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        assert!(path.starts_with("runtime/temp/clean_on_exit/"));
        Ok(Box::new(FailingSession(self.0.clone())))
    }
}

impl RuntimeStorageWriteSession for FailingSession {
    fn writeChunk(&mut self, _chunk: &[u8]) -> HostResult<()> {
        Err(HostError::new("simulated disk full"))
    }
    fn commit(self: Box<Self>) -> HostResult<()> {
        panic!("a failed write must not commit")
    }
    fn discard(self: Box<Self>) -> HostResult<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(HostError::new("simulated cleanup failure"))
    }
}

/// Isolates application singletons so every database handle closes before directory cleanup.
#[test]
fn streamed_attachments_publish_only_complete_bounded_uploads() {
    let temporaryRoot = std::env::temp_dir().canonicalize().unwrap();
    let root = temporaryRoot.join(format!(
        "operit-attachment-stream-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "streamed_attachment_worker", "--ignored", "--nocapture"])
        .env("OPERIT_STREAMED_ATTACHMENT_TEST_ROOT", &root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "upload worker failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let resolvedRoot = root.canonicalize().unwrap();
    assert_eq!(resolvedRoot.parent(), Some(temporaryRoot.as_path()));
    std::fs::remove_dir_all(resolvedRoot).unwrap();
}

/// Exercises the shared input stream and real Host writer inside the parent-owned test process.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Executed by the parent test with an explicitly owned storage directory"]
async fn streamed_attachment_worker() {
    let root = std::path::PathBuf::from(
        std::env::var_os("OPERIT_STREAMED_ATTACHMENT_TEST_ROOT")
            .expect("upload worker requires its parent-owned storage directory"),
    );
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let scheduler = Arc::new(NativeHostRuntimeTaskSchedulerHost::new());
    setDefaultHostRuntimeTaskSchedulerHost(scheduler.clone());
    let host = HostManager {
        fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
        hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
        runtimeStorageHost: Some(storage.clone()),
        runtimeStorageWriteHost: Some(storage.clone()),
        runtimeSqliteHost: Some(storage.clone()),
        hostRuntimeTaskSchedulerHost: Some(scheduler),
        ..HostManager::default()
    };
    let transfer = AttachmentTransferManager::getInstance(&host).unwrap();
    let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(host));
    let target =
        LocalCoreProxy::generatedTargetForSchema("services.attachmentTransferManager").unwrap();
    let size = 143_388_128i64;
    let id = transfer
        .beginAttachmentUpload("大文件.apk".into(), Some(size))
        .unwrap();
    let request = CorePushRequest::new("large-attachment", target, "writeAttachmentUpload")
        .withArgs(
            toCoreValue(
                json!({"uploadId": id, "fileName": "大文件.apk", "expectedByteLength": size}),
            )
            .unwrap(),
        );
    assert!(proxy.isReverseStreamRequest(&request));
    let client: &dyn CoreLinkSharedClient = &proxy;
    let mut stream = client.openPush(request).await.unwrap();
    tokio::time::timeout(Duration::from_secs(120), async {
        let mut sent = 0usize;
        let mut index = 0usize;
        while sent < size as usize {
            let count = CHUNK.min(size as usize - sent);
            stream
                .send(toCoreValue(vec![(index % 251) as u8; count]).unwrap())
                .await
                .unwrap();
            sent += count;
            index += 1;
        }
        stream.close().await.unwrap();
    })
    .await
    .expect("bounded upload must complete");
    let attachment = transfer
        .completeAttachmentUpload(id.clone(), "大文件.apk".into(), Some(size))
        .unwrap();
    assert_eq!(attachment.fileSize, size);
    assert_eq!(attachment.fileName, "大文件.apk");
    assert!(
        std::path::Path::new(&attachment.filePath)
            .starts_with(root.join("runtime/temp/clean_on_exit"))
    );
    let mut file = std::fs::File::open(&attachment.filePath).unwrap();
    let mut buffer = [0u8; 31];
    for index in [0usize, 1, 70, size as usize / CHUNK] {
        file.seek(SeekFrom::Start((index * CHUNK) as u64)).unwrap();
        file.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer, [(index % 251) as u8; 31]);
    }
    drop(file);
    transfer
        .discardAttachmentUpload(id, "大文件.apk".into())
        .unwrap();
    assert!(!std::path::Path::new(&attachment.filePath).exists());

    // Empty and unknown-size documents use the same writer, without Base64 or fallback routes.
    for expected in [Some(0), None] {
        let id = transfer
            .beginAttachmentUpload("empty".into(), expected)
            .unwrap();
        let (mut sender, input) = ReverseStream::<Vec<u8>>::channel();
        sender.close();
        transfer
            .writeAttachmentUpload(id.clone(), "empty".into(), expected, input)
            .await
            .unwrap();
        assert_eq!(
            transfer
                .completeAttachmentUpload(id.clone(), "empty".into(), Some(0))
                .unwrap()
                .fileSize,
            0
        );
        transfer
            .discardAttachmentUpload(id, "empty".into())
            .unwrap();
    }
    let id = transfer
        .beginAttachmentUpload("unknown".into(), None)
        .unwrap();
    let (mut sender, input) = ReverseStream::<Vec<u8>>::channel();
    sender.send(vec![0, 1, 255]).await.unwrap();
    sender.close();
    transfer
        .writeAttachmentUpload(id.clone(), "unknown".into(), None, input)
        .await
        .unwrap();
    let unknown = transfer
        .completeAttachmentUpload(id.clone(), "unknown".into(), Some(3))
        .unwrap();
    assert_eq!(std::fs::read(&unknown.filePath).unwrap(), [0, 1, 255]);
    transfer
        .discardAttachmentUpload(id, "unknown".into())
        .unwrap();

    // Incomplete, oversized and overlong writes must discard the private Host session.
    for (expected, content) in [
        (Some(10), vec![1; 3]),
        (Some(1), vec![1; 3]),
        (None, vec![1; CHUNK + 1]),
    ] {
        let id = transfer
            .beginAttachmentUpload("bad".into(), expected)
            .unwrap();
        let (mut sender, input) = ReverseStream::<Vec<u8>>::channel();
        sender.send(content).await.unwrap();
        sender.close();
        assert!(
            transfer
                .writeAttachmentUpload(id.clone(), "bad".into(), expected, input)
                .await
                .is_err()
        );
        assert!(
            transfer
                .completeAttachmentUpload(id.clone(), "bad".into(), None)
                .is_err()
        );
        transfer.discardAttachmentUpload(id, "bad".into()).unwrap();
    }
    assert!(
        transfer
            .beginAttachmentUpload("../escape.apk".into(), Some(1))
            .is_err()
    );
    assert!(
        transfer
            .beginAttachmentUpload("file.apk".into(), Some(-1))
            .is_err()
    );
    // Discard on Host I/O failure, preserving the write error if cleanup also fails.
    let discarded = Arc::new(AtomicUsize::new(0));
    let failing = AttachmentTransferManager::getInstance(&HostManager {
        runtimeStorageHost: Some(storage.clone()),
        runtimeStorageWriteHost: Some(Arc::new(FailingWriteHost(discarded.clone()))),
        fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
        ..HostManager::default()
    })
    .unwrap();
    let id = failing
        .beginAttachmentUpload("disk-full".into(), Some(3))
        .unwrap();
    let (mut sender, input) = ReverseStream::<Vec<u8>>::channel();
    sender.send(vec![0, 1, 255]).await.unwrap();
    sender.close();
    let error = failing
        .writeAttachmentUpload(id, "disk-full".into(), Some(3), input)
        .await
        .unwrap_err();
    assert!(error.contains("simulated disk full"));
    assert_eq!(discarded.load(Ordering::SeqCst), 1);

    // Native write-session staging files must also have been removed on every failure.
    assert_eq!(
        std::fs::read_dir(root.join("runtime/temp/clean_on_exit"))
            .unwrap()
            .count(),
        0
    );
    // The parent removes the directory after this process releases all application singletons.
}
