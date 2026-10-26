use operit_host_api::{HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::RuntimeFileSyncStore::{RuntimeFileSyncStore, RUNTIME_FILE_SYNC_DOMAIN};
use operit_store::SyncOperationStore::{SyncClock, SyncOperation, SyncOperationStore};
use operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const SYNC: &str = "runtime/sync";
struct Node {
    root: std::path::PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
}
impl Node {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-ws-sync-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("workspaces")).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self { root, host }
    }
    fn workspace(&self) -> WorkspaceFileSyncStore {
        WorkspaceFileSyncStore::new(self.host.clone(), SYNC)
    }
    fn log(&self) -> SyncOperationStore {
        SyncOperationStore::new(self.host.clone(), SYNC)
    }
    fn ops(&self) -> Vec<SyncOperation> {
        self.log()
            .operationsSince(
                &SyncClock::empty(),
                &[RUNTIME_FILE_SYNC_DOMAIN.into()],
                1000,
            )
            .unwrap()
    }
    fn send(&self, to: &Node) {
        let clock = to.log().localClock().unwrap();
        let ops = self
            .log()
            .operationsSince(&clock, &[RUNTIME_FILE_SYNC_DOMAIN.into()], 1000)
            .unwrap();
        for op in ops {
            self.deliver(to, &op);
        }
    }
    fn deliver(&self, to: &Node, op: &SyncOperation) {
        if let Some(reference) = RuntimeFileSyncStore::requiredBlob(op).unwrap() {
            let source = RuntimeFileSyncStore::new(self.host.clone(), SYNC);
            let path = source.blobStoragePath(&reference).unwrap();
            RuntimeFileSyncStore::new(to.host.clone(), SYNC)
                .writeBlob(&reference, &self.host.readBytes(&path).unwrap())
                .unwrap();
        }
        to.workspace().applyOperation(op, false).unwrap();
    }
    fn write(&self, path: &str, bytes: &[u8]) {
        self.host.writeBytes(path, bytes).unwrap();
    }
}
impl Drop for Node {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn bidirectional_binary_empty_and_no_echo() {
    let a = Node::new();
    let b = Node::new();
    a.write("workspaces/project/中文.txt", b"hello");
    a.write("workspaces/project/image.bin", &[0, 255, 17, 32]);
    a.write("workspaces/project/empty.txt", b"");
    assert_eq!(a.workspace().scan().unwrap(), 3);
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/image.bin").unwrap(),
        [0, 255, 17, 32]
    );
    assert_eq!(b.workspace().scan().unwrap(), 0);
    assert_eq!(b.ops().len(), 3);
    std::thread::sleep(std::time::Duration::from_millis(2));
    b.write("workspaces/project/中文.txt", b"phones");
    b.workspace().scan().unwrap();
    b.send(&a);
    assert_eq!(
        a.host.readBytes("workspaces/project/中文.txt").unwrap(),
        b"phones"
    );
    assert_eq!(a.workspace().scan().unwrap(), 0);
    assert_eq!(a.ops().len(), 4);
}

#[test]
fn offline_rename_recursive_delete_and_restart_inventory() {
    let a = Node::new();
    let b = Node::new();
    a.write("workspaces/project/sub/a.txt", b"one");
    a.write("workspaces/project/sub/b.txt", b"two");
    a.workspace().scan().unwrap();
    a.send(&b);
    // No service instance survives these external edits.
    std::fs::rename(
        a.root.join("workspaces/project/sub/a.txt"),
        a.root.join("workspaces/project/renamed.txt"),
    )
    .unwrap();
    std::fs::remove_dir_all(a.root.join("workspaces/project/sub")).unwrap();
    assert_eq!(a.workspace().scan().unwrap(), 3);
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/renamed.txt").unwrap(),
        b"one"
    );
    assert!(!b.host.exists("workspaces/project/sub/a.txt").unwrap());
    assert!(!b.host.exists("workspaces/project/sub/b.txt").unwrap());
    assert_eq!(b.workspace().scan().unwrap(), 0);
}

#[test]
fn simultaneous_changes_converge_and_external_mount_is_excluded() {
    let a = Node::new();
    let b = Node::new();
    a.write("runtime/local.txt", b"not shared");
    a.write("workspaces/project/main.txt", b"desktop");
    b.write("workspaces/project/main.txt", b"phone");
    a.workspace().scan().unwrap();
    b.workspace().scan().unwrap();
    let a_op = a.ops().pop().unwrap();
    let b_op = b.ops().pop().unwrap();
    a.deliver(&b, &a_op);
    b.deliver(&a, &b_op);
    assert_eq!(
        a.host.readBytes("workspaces/project/main.txt").unwrap(),
        b.host.readBytes("workspaces/project/main.txt").unwrap()
    );
    assert_eq!(a.workspace().scan().unwrap(), 0);
    assert_eq!(b.workspace().scan().unwrap(), 0);
    assert!(!b.host.exists("runtime/local.txt").unwrap());
    assert_eq!(a.ops().len(), 2);
}

#[cfg(unix)]
#[test]
fn links_are_not_uploaded_and_remote_writes_cannot_follow_links() {
    use std::os::unix::fs::symlink;
    let a = Node::new();
    let b = Node::new();
    let outside = a.root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), b"private").unwrap();
    std::fs::create_dir_all(a.root.join("workspaces/project")).unwrap();
    symlink(&outside, a.root.join("workspaces/project/mount")).unwrap();
    symlink(
        a.root.join("workspaces/project"),
        a.root.join("workspaces/project/loop"),
    )
    .unwrap();
    symlink(
        outside.join("secret.txt"),
        a.root.join("workspaces/project/file-link"),
    )
    .unwrap();
    symlink(
        outside.join("missing"),
        a.root.join("workspaces/project/broken"),
    )
    .unwrap();
    assert_eq!(a.workspace().scan().unwrap(), 0);
    assert!(a.ops().is_empty());
    b.write("workspaces/project/broken", b"must not escape");
    b.workspace().scan().unwrap();
    let op = b.ops().pop().unwrap();
    let reference = RuntimeFileSyncStore::requiredBlob(&op).unwrap().unwrap();
    RuntimeFileSyncStore::new(a.host.clone(), SYNC)
        .writeBlob(&reference, b"must not escape")
        .unwrap();
    assert!(a.workspace().applyOperation(&op, false).is_err());
    assert!(!outside.join("missing").exists());
    assert_eq!(
        std::fs::read(outside.join("secret.txt")).unwrap(),
        b"private"
    );
}

#[test]
fn invalid_paths_are_rejected() {
    let a = Node::new();
    for path in [
        "workspaces/../outside",
        "workspaces/x/../../secret",
        "/mnt/macos/file",
        "workspaces/x\\outside",
        "workspaces/x//file",
    ] {
        assert!(
            a.workspace()
                .apply(path, "delete", serde_json::Value::Null)
                .is_err(),
            "{path}"
        );
    }
}

#[test]
fn space_join_reexports_existing_local_files_without_deleting_them() {
    let a = Node::new();
    let b = Node::new();
    a.write("workspaces/phone/notes.txt", b"keep me");
    a.workspace().scan().unwrap();
    a.log().markLocalOperationsUnexportable().unwrap();
    assert!(a.ops().is_empty());
    a.workspace().prepareSpaceJoin().unwrap();
    a.workspace().scan().unwrap();
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/phone/notes.txt").unwrap(),
        b"keep me"
    );
}

#[test]
fn missing_root_does_not_publish_mass_deletions() {
    let a = Node::new();
    a.write("workspaces/project/keep.txt", b"keep me");
    a.workspace().scan().unwrap();
    std::fs::rename(a.root.join("workspaces"), a.root.join("unavailable")).unwrap();
    assert!(a.workspace().scan().is_err());
    assert_eq!(a.ops().len(), 1);
}

#[test]
fn direct_workspace_repository_writes_do_not_echo_or_duplicate() {
    let a = Node::new();
    let b = Node::new();
    let files = RuntimeFileSyncStore::new(a.host.clone(), SYNC);
    files
        .writeBytes("workspaces/project/direct.txt", b"one")
        .unwrap();
    assert_eq!(a.workspace().scan().unwrap(), 0);
    a.send(&b);
    files
        .appendBytes("workspaces/project/direct.txt", b" two")
        .unwrap();
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/direct.txt").unwrap(),
        b"one two"
    );
    files.delete("workspaces/project/direct.txt").unwrap();
    a.send(&b);
    assert!(!b.host.exists("workspaces/project/direct.txt").unwrap());
    // operationsSince compacts superseded states of the same entity.
    assert_eq!(a.ops().len(), 1);
    assert_eq!(a.workspace().scan().unwrap(), 0);
    assert_eq!(b.workspace().scan().unwrap(), 0);
}

#[test]
fn application_batch_does_not_skip_other_domains_before_a_workspace_operation() {
    use operit_store::SyncOperationStore::{NewSyncOperation, SyncOperationSemantics};
    let a = Node::new();
    let b = Node::new();
    let preferences = a
        .log()
        .appendLocalOperation(
            &a.log().localDeviceId().unwrap(),
            NewSyncOperation {
                domain: "preferences".into(),
                entityType: "test".into(),
                entityId: "preferences-before-workspace".into(),
                operation: "set".into(),
                semantics: SyncOperationSemantics::EntityState,
                payload: serde_json::Value::Null,
            },
        )
        .unwrap();
    a.write("workspaces/project/one.txt", b"one");
    a.workspace().scan().unwrap();
    let file = a.ops().pop().unwrap();
    let reference = RuntimeFileSyncStore::requiredBlob(&file).unwrap().unwrap();
    RuntimeFileSyncStore::new(b.host.clone(), SYNC)
        .writeBlob(&reference, b"one")
        .unwrap();
    b.workspace().materializeOperation(&file, false).unwrap();
    assert_eq!(
        b.log()
            .localClock()
            .unwrap()
            .sequenceFor(&file.originDeviceId),
        0
    );
    b.log().appendOperations(&[preferences, file]).unwrap();
    let operations = b
        .log()
        .operationsSince(&SyncClock::empty(), &[], 100)
        .unwrap();
    assert_eq!(operations.len(), 2);
}

#[test]
fn file_to_directory_and_back_replicate_in_order() {
    let a = Node::new();
    let b = Node::new();
    a.write("workspaces/project/item", b"file");
    a.workspace().scan().unwrap();
    a.send(&b);
    a.host.delete("workspaces/project/item", false).unwrap();
    a.write("workspaces/project/item/child", b"child");
    a.workspace().scan().unwrap();
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/item/child").unwrap(),
        b"child"
    );
    a.host.delete("workspaces/project/item", true).unwrap();
    a.write("workspaces/project/item", b"file again");
    a.workspace().scan().unwrap();
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/item").unwrap(),
        b"file again"
    );
}

/// Counts workspace file reads so tests can assert what a scan actually read.
struct CountingHost {
    inner: Arc<NativeRuntimeStorageHost>,
    workspaceReads: AtomicUsize,
}

impl RuntimeStorageHost for CountingHost {
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        self.inner.runtimeRootDir()
    }
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        self.inner.workspaceRootDir()
    }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        if path.starts_with("workspaces/") {
            self.workspaceReads.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.readBytes(path)
    }
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.inner.readBytesRange(path, offset, length)
    }
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytesAtomically(path, content)
    }
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.appendBytes(path, content)
    }
    fn flushFile(&self, path: &str) -> HostResult<()> {
        self.inner.flushFile(path)
    }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

struct CountedNode {
    root: std::path::PathBuf,
    host: Arc<CountingHost>,
}

impl CountedNode {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "operit-ws-sync-counted-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("workspaces")).unwrap();
        let inner = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self {
            root,
            host: Arc::new(CountingHost {
                inner,
                workspaceReads: AtomicUsize::new(0),
            }),
        }
    }
    fn workspace(&self) -> WorkspaceFileSyncStore {
        WorkspaceFileSyncStore::new(self.host.clone(), SYNC)
    }
    fn reads(&self) -> usize {
        self.host.workspaceReads.load(Ordering::SeqCst)
    }
    /// Backdates the persisted inventory so the next scan must run a full-hash round.
    fn expireFullHash(&self) {
        const STATE: &str = "runtime/sync/workspace_inventory.json";
        let mut state: serde_json::Value =
            serde_json::from_slice(&self.host.readBytes(STATE).unwrap()).unwrap();
        let now = operit_host_api::TimeUtils::tryCurrentTimeMillis().unwrap();
        state["lastFullHashAt"] = serde_json::json!(now - 6 * 60_000);
        self.host
            .writeBytes(STATE, &serde_json::to_vec(&state).unwrap())
            .unwrap();
    }
}

impl Drop for CountedNode {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn size_unchanged_scans_reuse_references_without_reading() {
    let node = CountedNode::new();
    node.host
        .writeBytes("workspaces/project/file.txt", b"content")
        .unwrap();
    assert_eq!(node.workspace().scan().unwrap(), 1); // initial inventory round
    let baseline = node.reads();
    assert_eq!(node.workspace().scan().unwrap(), 0);
    assert_eq!(node.reads(), baseline); // steady state: no reads, no hashing
}

#[test]
fn size_change_triggers_exactly_one_read() {
    let node = CountedNode::new();
    node.host
        .writeBytes("workspaces/project/file.txt", b"v1")
        .unwrap();
    node.workspace().scan().unwrap();
    node.host
        .writeBytes("workspaces/project/file.txt", b"version two")
        .unwrap();
    let baseline = node.reads();
    assert_eq!(node.workspace().scan().unwrap(), 1);
    assert_eq!(node.reads() - baseline, 1);
}

#[test]
fn same_size_external_edit_waits_for_the_full_hash_round() {
    let node = CountedNode::new();
    node.host
        .writeBytes("workspaces/project/file.txt", b"hello")
        .unwrap();
    node.workspace().scan().unwrap();
    // Same-length in-place rewrite made by an external editor or terminal.
    node.host
        .writeBytes("workspaces/project/file.txt", b"world")
        .unwrap();
    assert_eq!(node.workspace().scan().unwrap(), 0);
    assert_eq!(
        node.host.readBytes("workspaces/project/file.txt").unwrap(),
        b"world"
    );
    // The periodic full-hash round discovers it and publishes exactly once.
    node.expireFullHash();
    assert_eq!(node.workspace().scan().unwrap(), 1);
    assert_eq!(node.workspace().scan().unwrap(), 0);
}

#[test]
fn legacy_inventory_without_last_full_hash_round_hashes_every_file() {
    let node = CountedNode::new();
    node.host
        .writeBytes("workspaces/project/a.txt", b"one")
        .unwrap();
    node.host
        .writeBytes("workspaces/project/b.txt", b"two")
        .unwrap();
    node.workspace().scan().unwrap();
    // Inventories written by older builds carry no lastFullHashAt field; the
    // first scan on the new build must hash everything before trusting sizes.
    const STATE: &str = "runtime/sync/workspace_inventory.json";
    let mut state: serde_json::Value =
        serde_json::from_slice(&node.host.readBytes(STATE).unwrap()).unwrap();
    state.as_object_mut().unwrap().remove("lastFullHashAt");
    node.host
        .writeBytes(STATE, &serde_json::to_vec(&state).unwrap())
        .unwrap();
    let baseline = node.reads();
    assert_eq!(node.workspace().scan().unwrap(), 0); // no changes, but…
    assert_eq!(node.reads() - baseline, 2); // …every file was re-hashed
}

#[test]
fn structured_same_size_writes_publish_immediately() {
    let a = Node::new();
    let b = Node::new();
    a.write("workspaces/project/file.txt", b"hello");
    a.workspace().scan().unwrap();
    // RuntimeFileSyncStore routes workspace writes through trackPaths, so the
    // same-size rewrite must not wait for the periodic full-hash round.
    RuntimeFileSyncStore::new(a.host.clone(), SYNC)
        .writeBytes("workspaces/project/file.txt", b"world")
        .unwrap();
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/file.txt").unwrap(),
        b"world"
    );
}
