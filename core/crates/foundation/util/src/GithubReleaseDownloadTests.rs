use super::*;
use crate::RuntimeStorageLayout::RUNTIME_APPLICATION_UPDATES_DIR_PATH;
use crate::UpdateHttp::tests::{assertPending, head, ControlledHttpHost};
use operit_host_api::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct Clock(AtomicUsize);
impl HostRuntimeTaskSchedulerHost for Clock {
    /// Advances a deterministic Host clock without scheduling update work.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(self.0.fetch_add(10, Ordering::Relaxed) as u64)
    }
    /// Rejects reintroduction of a synchronous task wrapper.
    fn scheduleHostRuntimeTask(&self, _: &str, _: HostRuntimeTask) -> HostResult<()> {
        panic!("no update task wrapper")
    }
    /// Rejects reintroduction of an asynchronous task wrapper.
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> {
        panic!("no update task wrapper")
    }
    /// Rejects hidden blocking execution in the common update path.
    fn scheduleHostRuntimeBlockingAsyncTask(
        &self,
        _: &str,
        _: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        panic!("no blocking wrapper")
    }
    /// Rejects polling as a substitute for callback completion.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        _: &str,
        _: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        panic!("no update task wrapper")
    }
    /// Rejects timer-based callback polling.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        panic!("no update timer")
    }
    /// Rejects callback polling through runtime turns.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        panic!("no update polling")
    }
    /// Rejects callback polling through delays.
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        panic!("no update polling")
    }
}

#[derive(Clone, Default)]
struct MemoryStorage {
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    pending: Arc<AtomicUsize>,
    discarded: Arc<AtomicUsize>,
}

impl RuntimeStorageHost for MemoryStorage {
    /// Proves update downloads do not depend on a physical runtime root.
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    /// Proves update downloads do not depend on a physical workspace root.
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    /// Reads only committed logical bytes.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| HostError::new("missing fixture file"))
    }
    /// Writes fixture files into the virtual storage map.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.files
            .lock()
            .unwrap()
            .insert(path.into(), content.to_vec());
        Ok(())
    }
    /// Appends fixture bytes without exposing a native path.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.files
            .lock()
            .unwrap()
            .entry(path.into())
            .or_default()
            .extend_from_slice(content);
        Ok(())
    }
    /// Memory writes have no separate device flush.
    fn flushFile(&self, _: &str) -> HostResult<()> {
        Ok(())
    }
    /// Deletes only this exact virtual entry and its descendants.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        let prefix = format!("{path}/");
        self.files
            .lock()
            .unwrap()
            .retain(|key, _| key != path && !(recursive && key.starts_with(&prefix)));
        Ok(())
    }
    /// Checks logical entries and directories without a filesystem.
    fn exists(&self, path: &str) -> HostResult<bool> {
        assert!(path.starts_with(RUNTIME_APPLICATION_UPDATES_DIR_PATH));
        let prefix = format!("{path}/");
        Ok(self
            .files
            .lock()
            .unwrap()
            .keys()
            .any(|key| key == path || key.starts_with(&prefix)))
    }
    /// Lists immediate committed children while private write sessions remain invisible.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let mut entries = BTreeMap::new();
        for (path, bytes) in self.files.lock().unwrap().iter() {
            if let Some(relative) = path.strip_prefix(&format!("{prefix}/")) {
                let (name, isDirectory) = match relative.split_once('/') {
                    Some((name, _)) => (name, true),
                    None => (relative, false),
                };
                entries.insert(
                    name.to_string(),
                    RuntimeStorageEntry {
                        path: format!("{prefix}/{name}"),
                        isDirectory,
                        size: if isDirectory { 0 } else { bytes.len() as i64 },
                    },
                );
            }
        }
        Ok(entries.into_values().collect())
    }
}

struct MemoryWriteSession {
    storage: MemoryStorage,
    path: String,
    bytes: Vec<u8>,
}
impl Drop for MemoryWriteSession {
    /// Tracks every released session independently of publication or explicit discard.
    fn drop(&mut self) {
        self.storage.pending.fetch_sub(1, Ordering::SeqCst);
    }
}
impl RuntimeStorageWriteSession for MemoryWriteSession {
    /// Keeps incoming chunks private until commit.
    fn writeChunk(&mut self, chunk: &[u8]) -> HostResult<()> {
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }
    /// Publishes the entire package in a single logical storage operation.
    fn commit(self: Box<Self>) -> HostResult<()> {
        self.storage.writeBytes(&self.path, &self.bytes)
    }
    /// Records explicit cleanup so tests detect reliance on native-only destructors.
    fn discard(self: Box<Self>) -> HostResult<()> {
        self.storage.discarded.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
impl RuntimeStorageWriteHost for MemoryStorage {
    /// Opens an isolated logical writer without creating native directories.
    fn createWriteSession(&self, path: &str) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        assert!(path.starts_with(&format!("{RUNTIME_APPLICATION_UPDATES_DIR_PATH}/")));
        self.pending.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(MemoryWriteSession {
            storage: self.clone(),
            path: path.into(),
            bytes: Vec::new(),
        }))
    }
}

struct Fixture {
    storage: Arc<MemoryStorage>,
    http: Arc<ControlledHttpHost>,
    context: HostManager,
}
impl Fixture {
    /// Combines virtual storage with controlled HTTP and a clock that rejects task scheduling.
    fn new() -> Self {
        let storage = Arc::new(MemoryStorage::default());
        let http = Arc::new(ControlledHttpHost::default());
        let context = HostManager::new()
            .withRuntimeStorageHost(storage.clone())
            .withRuntimeStorageWriteHost(storage.clone())
            .withHttpHost(http.clone())
            .withHostRuntimeTaskSchedulerHost(Arc::new(Clock::default()));
        storage
            .writeBytes(
                &format!("{RUNTIME_APPLICATION_UPDATES_DIR_PATH}/other-task/keep"),
                b"keep",
            )
            .unwrap();
        Self {
            storage,
            http,
            context,
        }
    }

    /// Confirms failure cleanup leaves another update task's storage untouched.
    fn assertOnlyOtherTask(&self) {
        assert_eq!(self.storage.pending.load(Ordering::SeqCst), 0);
        let children = self
            .storage
            .list(RUNTIME_APPLICATION_UPDATES_DIR_PATH)
            .unwrap();
        assert_eq!(
            children.len(),
            1,
            "unexpected update attempt remains: {children:?}"
        );
        assert_eq!(
            self.storage
                .readBytes(&format!(
                    "{RUNTIME_APPLICATION_UPDATES_DIR_PATH}/other-task/keep"
                ))
                .unwrap(),
            b"keep"
        );
    }
}

const NAME: &str = "operit2-app-windows-x86_64-2.0.0.msi";
const MSI: &[u8] = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1payload";

/// Uses a valid project release URL while keeping networking under fixture control.
fn packageUrl() -> String {
    format!("https://github.com/AAswordman/Operit2/releases/download/v2.0.0/{NAME}")
}

/// Commits only after complete, valid streamed bytes and returns a virtual storage path.
#[tokio::test]
async fn publishes_verified_package_after_clean_completion() {
    let fixture = Fixture::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let progress = events.clone();
    let mut future = Box::pin(GithubReleaseUtil::downloadAndPrepareFullUpdateWithProgress(
        packageUrl(),
        NAME.into(),
        &fixture.context,
        move |event| progress.lock().unwrap().push(event),
    ));
    assertPending(future.as_mut());
    let callbacks = fixture.http.callbacks();
    (callbacks.head)(head(MSI.len()));
    for chunk in MSI.chunks(3) {
        (callbacks.bytes)(chunk.to_vec());
    }
    assert!(!events.lock().unwrap().iter().any(|e| matches!(
        e,
        FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::Ready,
            ..
        }
    )));
    (callbacks.closed)(Ok(()));
    let path = future.await.unwrap();
    assert!(path.starts_with(&format!("{RUNTIME_APPLICATION_UPDATES_DIR_PATH}/")));
    assert_eq!(
        crate::RuntimeStorageLayout::runtimeStorageOwnership(&path).unwrap(),
        crate::RuntimeStorageLayout::RuntimeStorageOwnership::Ephemeral
    );
    assert_eq!(fixture.storage.readBytes(&path).unwrap(), MSI);
    assert!(events.lock().unwrap().iter().any(|e| matches!(e, FullUpdateProgressEvent::DownloadProgress { readBytes, totalBytes, .. } if *readBytes == MSI.len() as u64 && *totalBytes == MSI.len() as u64)));
    assert!(matches!(
        events.lock().unwrap().last(),
        Some(FullUpdateProgressEvent::StageChanged {
            stage: FullUpdateStage::Ready,
            ..
        })
    ));
}

/// Failed responses never publish a file or remove another task's directory.
#[tokio::test]
async fn rejects_truncated_html_oversized_and_failed_downloads() {
    for scenario in 0..6 {
        let fixture = Fixture::new();
        let mut future = Box::pin(GithubReleaseUtil::downloadAndPrepareFullUpdateWithProgress(
            packageUrl(),
            NAME.into(),
            &fixture.context,
            |_| {},
        ));
        assertPending(future.as_mut());
        let callbacks = fixture.http.callbacks();
        let mut response = head(MSI.len());
        if scenario == 0 {
            response.headers[0].1 = (MSI.len() + 1).to_string();
        }
        if scenario == 2 {
            response.headers[0].1 = (MSI.len() - 1).to_string();
        }
        if scenario == 4 {
            response.headers.clear();
        }
        if scenario == 5 {
            response.statusCode = 403;
        }
        (callbacks.head)(response);
        (callbacks.bytes)(if scenario == 1 {
            vec![b'<'; MSI.len()]
        } else {
            MSI.to_vec()
        });
        (callbacks.closed)(if scenario == 3 {
            Err("connection lost".into())
        } else {
            Ok(())
        });
        assert!(future.await.is_err(), "scenario {scenario}");
        fixture.assertOnlyOtherTask();
        assert_eq!(fixture.storage.discarded.load(Ordering::SeqCst), 1);
    }
}

/// Cancellation at headers, in progress and before opening all discard the write session.
#[tokio::test]
async fn cancellation_and_dropped_future_discard_only_their_attempt() {
    for scenario in 0..4 {
        let fixture = Fixture::new();
        let control = HttpDownloadControl::new();
        if scenario == 0 {
            control.cancel();
        }
        let cancel = control.clone();
        let mut future = Box::pin(GithubReleaseUtil::downloadAndPrepareFullUpdateControlled(
            packageUrl(),
            NAME.into(),
            &fixture.context,
            control.clone(),
            move |event| {
                if scenario == 2
                    && matches!(event, FullUpdateProgressEvent::DownloadProgress { .. })
                {
                    cancel.cancel();
                }
            },
        ));
        if scenario == 0 {
            assert_eq!(future.await.unwrap_err(), "UPDATE_CANCELLED");
        } else {
            assertPending(future.as_mut());
            if scenario == 2 {
                let callbacks = fixture.http.callbacks();
                (callbacks.head)(head(MSI.len()));
                (callbacks.bytes)(MSI[..4].to_vec());
            } else if scenario == 1 {
                control.cancel();
            }
            if scenario == 3 {
                drop(future);
            } else {
                assert_eq!(future.await.unwrap_err(), "UPDATE_CANCELLED");
            }
            assert!(fixture.http.closed.load(Ordering::SeqCst));
        }
        fixture.assertOnlyOtherTask();
        assert_eq!(
            fixture.storage.discarded.load(Ordering::SeqCst),
            usize::from(scenario != 0)
        );
    }
}
