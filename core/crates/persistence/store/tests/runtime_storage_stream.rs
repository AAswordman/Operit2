//! Bounded asset transport preserves the repository's commit and sync semantics.
use base64::{engine::general_purpose::STANDARD, Engine};
use operit_host_api::{HostManager::setDefaultHostRuntimeTaskSchedulerHost, RuntimeStorageHost};
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    repository::RuntimeStorageRepository::RuntimeStorageRepository,
    CoreNodeIdentityStore::CoreNodeIdentityStore,
    RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost},
};
use operit_util::stream::ReverseStream::ReverseStream;
use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

struct ThreadWake(std::thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}
fn run<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}
fn upload(
    store: &RuntimeStorageRepository,
    path: &str,
    length: i64,
    chunks: Vec<Vec<u8>>,
) -> Result<(), String> {
    let (mut sender, stream) = ReverseStream::channel();
    let producer = std::thread::spawn(move || {
        run(async move {
            for chunk in chunks {
                if sender.send(chunk).await.is_err() {
                    break;
                }
            }
            sender.close();
        })
    });
    let result = run(store.writeBytesStream(path.into(), length, stream));
    producer.join().unwrap();
    result
}

#[test]
fn bounded_uploads_and_reads_preserve_complete_fonts_and_reject_incomplete_writes() {
    let root = std::env::temp_dir().join(format!("operit-font-stream-{}", uuid::Uuid::new_v4()));
    let host = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host.clone());
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    CoreNodeIdentityStore::new(host.clone())
        .writeNodeId("font-source".into())
        .unwrap();
    let store = RuntimeStorageRepository::new();
    let path = format!("{}/large.ttf", store.themeAssetsDirPath());
    let content = vec![0x5a; 1024 * 1024 + 29];
    upload(
        &store,
        &path,
        content.len() as i64,
        content.chunks(1024 * 1024).map(Vec::from).collect(),
    )
    .unwrap();
    assert_eq!(host.readBytes(&path).unwrap(), content);
    assert!(host.exists("runtime/sync").unwrap());
    assert_eq!(
        store
            .readBase64Chunk(path.clone(), 1024 * 1024, 1024 * 1024)
            .unwrap(),
        Some(STANDARD.encode([0x5a; 29]))
    );
    assert_eq!(
        store
            .readBase64Chunk(path.clone(), content.len() as i64, 1024)
            .unwrap(),
        Some(String::new())
    );
    assert_eq!(
        store
            .readBase64Chunk(format!("{path}.missing"), 0, 1024)
            .unwrap(),
        None
    );
    for (length, chunks) in [
        (100, vec![vec![1; 99]]),
        (1, vec![vec![1; 2]]),
        (1024 * 1024 + 1, vec![vec![1; 1024 * 1024 + 1]]),
        (-1, vec![]),
        (256 * 1024 * 1024 + 1, vec![]),
    ] {
        assert!(upload(&store, &path, length, chunks).is_err());
        assert_eq!(
            host.readBytes(&path).unwrap(),
            content,
            "failed upload must preserve the previous font"
        );
    }
    assert!(store.readBase64Chunk(path.clone(), -1, 1).is_err());
    assert!(store.readBase64Chunk(path, 0, 1024 * 1024 + 1).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
