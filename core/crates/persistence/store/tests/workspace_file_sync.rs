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
            RuntimeFileSyncStore::new(to.host.clone(), SYNC)
                .writeBlob(&reference, &source.readBlob(&reference).unwrap())
                .unwrap();
        }
        if WorkspaceFileSyncStore::owns(op) {
            to.workspace().applyOperation(op, false).unwrap();
        } else {
            if to.log().shouldApplyOperation(op).unwrap() {
                RuntimeFileSyncStore::applySyncedOperation(
                    to.host.clone(), SYNC, &op.entityId, &op.operation, op.payload.clone(),
                ).unwrap();
                to.log().recordAppliedOperation(op).unwrap();
            }
            to.log().appendOperation(op).unwrap();
        }
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

/// Extracted database trees are ordinary files through capture, delivery and later edits.
#[test]
fn copied_database_trees_sync_without_policy_or_repeated_inventory_changes() {
    let a = Node::new();
    let b = Node::new();
    let mut copied_files = Vec::new();
    for root in [
        "workspaces/project",
        "runtime/data/chat_outputs/chat",
        "runtime/data/chat_attachments/chat",
    ] {
        let database = format!("{root}/identities/old/runtime/plugin_data/space/demo/records.sqlite");
        let marker = format!("{database}.operit-storage.json");
        let metadata = br#"{"path":"runtime/plugin_data/space/demo/records.sqlite","owner":"demo","kind":"sqlite","sync":true}"#;
        for (path, bytes) in [
            (database, b"copied database".as_slice()),
            (marker, metadata.as_slice()),
            (format!("{root}/broken.sqlite"), b"another copied file".as_slice()),
            (format!("{root}/broken.sqlite.operit-storage.json"), b"invalid copied metadata".as_slice()),
        ] {
            a.write(&path, bytes);
            copied_files.push((path, bytes.to_vec()));
        }
        assert!(!a.host.exists(&format!("{root}/.operit/sync.json")).unwrap());
    }
    a.write("workspaces/healthy/note.txt", b"independent user file");
    assert_eq!(a.workspace().scan().unwrap(), copied_files.len() + 1);
    a.send(&b);
    for (path, bytes) in &copied_files {
        assert_eq!(b.host.readBytes(path).unwrap(), *bytes, "{path}");
    }
    assert_eq!(b.host.readBytes("workspaces/healthy/note.txt").unwrap(), b"independent user file");
    for node in [&a, &b] {
        let history = node.ops();
        let versions = node.host.readBytes("runtime/sync/entity_versions.jsonl").unwrap();
        for _ in 0..5 {
            // Each call creates a fresh store, including reloading its durable inventory.
            assert_eq!(node.workspace().scan().unwrap(), 0);
        }
        assert_eq!(node.ops(), history);
        assert_eq!(node.host.readBytes("runtime/sync/entity_versions.jsonl").unwrap(), versions);
    }

    let path = &copied_files[0].0;
    b.workspace().trackPaths(&[path], || {
        b.host.writeBytes(path, b"edited copy").map_err(|error| error.to_string())
    }).unwrap();
    b.workspace().scan().unwrap();
    b.send(&a);
    assert_eq!(a.host.readBytes(path).unwrap(), b"edited copy");
    assert_eq!(a.workspace().scan().unwrap(), 0);
    assert_eq!(b.workspace().scan().unwrap(), 0);

    let output = "runtime/data/chat_outputs/chat/identities";
    a.workspace().trackPaths(&[output], || {
        a.host.delete(output, true).map_err(|error| error.to_string())
    }).unwrap();
    a.workspace().scan().unwrap();
    a.send(&b);
    for suffix in ["records.sqlite", "records.sqlite.operit-storage.json"] {
        let path = format!("{output}/old/runtime/plugin_data/space/demo/{suffix}");
        assert!(!a.host.exists(&path).unwrap());
        assert!(!b.host.exists(&path).unwrap());
    }
    assert_eq!(a.workspace().scan().unwrap(), 0);
    assert_eq!(b.workspace().scan().unwrap(), 0);
}

#[test]
fn bidirectional_binary_empty_and_no_echo() {
    let a = Node::new();
    let b = Node::new();
    a.write("workspaces/project/中文.txt", b"hello");
    a.write("workspaces/project/image.bin", &[0, 255, 17, 32]);
    a.write("workspaces/project/empty.txt", b"");
    a.host.setModifiedAt("workspaces/project/image.bin", 1_800_000_000_123_456).unwrap();
    assert_eq!(a.workspace().scan().unwrap(), 3);
    a.send(&b);
    assert_eq!(a.host.modifiedAt("workspaces/project/image.bin").unwrap(), b.host.modifiedAt("workspaces/project/image.bin").unwrap());
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
        "/virtual/macos/file",
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
    assert_eq!(a.workspace().scan().unwrap(), 1);
    a.send(&b);
    files
        .appendBytes("workspaces/project/direct.txt", b" two")
        .unwrap();
    a.workspace().scan().unwrap();
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/direct.txt").unwrap(),
        b"one two"
    );
    files.delete("workspaces/project/direct.txt").unwrap();
    a.workspace().scan().unwrap();
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
    inventoryWrites: AtomicUsize,
    failInventoryAt: AtomicUsize,
    failFlush: std::sync::atomic::AtomicBool,
    batchFlushes: AtomicUsize,
    directoryListings: std::sync::Mutex<Vec<String>>,
    removeBeforeMetadata: std::sync::Mutex<Option<String>>,
}

impl RuntimeStorageHost for CountingHost {
    fn openTransactionDatabase(&self, path: &str) -> HostResult<Box<dyn operit_host_api::RuntimeSqliteConnection>> {
        self.inner.openTransactionDatabase(path)
    }

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
        if path.starts_with("workspaces/") && offset == 0 { self.workspaceReads.fetch_add(1, Ordering::SeqCst); }
        self.inner.readBytesRange(path, offset, length)
    }
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }
    /// Reads metadata without counting a content read.
    fn modifiedAt(&self, path: &str) -> HostResult<i64> {
        let mut pending = self.removeBeforeMetadata.lock().unwrap();
        if pending.as_deref() == Some(path) {
            pending.take();
            self.inner.delete(path, false)?;
        }
        self.inner.modifiedAt(path)
    }
    /// Preserves source metadata.
    fn setModifiedAt(&self, path: &str, time: i64) -> HostResult<()> { self.inner.setModifiedAt(path, time) }
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> HostResult<()> {
        if path.ends_with("/workspace_inventory.json") {
            let write = self.inventoryWrites.fetch_add(1, Ordering::SeqCst) + 1;
            if write == self.failInventoryAt.load(Ordering::SeqCst) {
                return Err(operit_host_api::HostError::new("injected inventory checkpoint failure"));
            }
        }
        self.inner.writeBytesAtomically(path, content)
    }
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.appendBytes(path, content)
    }
    fn flushFile(&self, path: &str) -> HostResult<()> {
        self.inner.flushFile(path)
    }
    fn flushFiles(&self, paths: &[&str]) -> HostResult<()> {
        self.batchFlushes.fetch_add(1, Ordering::SeqCst);
        if self.failFlush.load(Ordering::SeqCst) {
            return Err(operit_host_api::HostError::new("injected file flush failure"));
        }
        self.inner.flushFiles(paths)
    }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.directoryListings.lock().unwrap().push(prefix.to_string());
        self.inner.list(prefix)
    }
}

struct CountedNode {
    host: Arc<CountingHost>,
    _node: Node,
}

impl CountedNode {
    /// Reuses the native node fixture while instrumenting only workspace content reads.
    fn new() -> Self {
        let node = Node::new();
        let host = Arc::new(CountingHost { inner: node.host.clone(), workspaceReads: AtomicUsize::new(0), inventoryWrites: AtomicUsize::new(0), failInventoryAt: AtomicUsize::new(0), failFlush: Default::default(), batchFlushes: AtomicUsize::new(0), directoryListings: Default::default(), removeBeforeMetadata: Default::default() });
        Self { host, _node: node }
    }
    fn workspace(&self) -> WorkspaceFileSyncStore {
        WorkspaceFileSyncStore::new(self.host.clone(), SYNC)
    }
    fn reads(&self) -> usize {
        self.host.workspaceReads.load(Ordering::SeqCst)
    }

}

/// A tracked write commits local bytes and intent without traversing unrelated trees.
#[test]
fn tracked_writes_leave_discovery_to_the_scanner() {
    let node = CountedNode::new();
    node.host.writeBytes("workspaces/unrelated/file.txt", b"old").unwrap();
    node.workspace().scan().unwrap();
    node.host.directoryListings.lock().unwrap().clear();
    let inventory_writes = node.host.inventoryWrites.load(Ordering::SeqCst);
    let reads = node.reads();
    RuntimeFileSyncStore::new(node.host.clone(), SYNC)
        .writeBytes("workspaces/project/file.txt", b"new").unwrap();
    assert!(node.host.directoryListings.lock().unwrap().is_empty());
    assert_eq!(node.host.inventoryWrites.load(Ordering::SeqCst), inventory_writes);
    assert_eq!(node.reads(), reads);
    assert_eq!(node.host.readBytes("workspaces/project/file.txt").unwrap(), b"new");
    assert!(node.host.exists("runtime/sync/workspace_dirty.json").unwrap());
    assert_eq!(node.workspace().scan().unwrap(), 1);
    assert!(!node.host.exists("runtime/sync/workspace_dirty.json").unwrap());
}

/// An invalid policy retains local intent until a later scan can safely publish it.
#[test]
fn failed_discovery_preserves_intent_across_store_recreation() {
    let node = Node::new();
    let path = "workspaces/project/file.txt";
    let policy = "workspaces/project/.operit/sync.json";
    node.write(policy, b"invalid JSON");
    node.workspace().trackPaths(&[path], || node.host.writeBytes(path, b"local").map_err(|e| e.to_string())).unwrap();
    node.workspace().scan().unwrap();
    assert!(node.host.exists("runtime/sync/workspace_dirty.json").unwrap());
    node.host.delete(policy, false).unwrap();
    let restarted = Arc::new(NativeRuntimeStorageHost::new(node.root.join("runtime"), node.root.join("workspaces")));
    WorkspaceFileSyncStore::new(restarted.clone(), SYNC).scan().unwrap();
    assert!(!restarted.exists("runtime/sync/workspace_dirty.json").unwrap());
    assert!(node.ops().iter().any(|operation| operation.entityId == path && operation.operation == "upsert"));
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
    let time = node.host.modifiedAt("workspaces/project/file.txt").unwrap();
    node.host
        .writeBytes("workspaces/project/file.txt", b"version two")
        .unwrap();
    node.host.setModifiedAt("workspaces/project/file.txt", time).unwrap();
    let baseline = node.reads();
    assert_eq!(node.workspace().scan().unwrap(), 1);
    assert_eq!(node.reads() - baseline, 1);
}

/// Same-size external edits are detected on the next metadata scan.
#[test]
fn same_size_external_edit_is_published_once() {
    let node = CountedNode::new();
    let path = "workspaces/project/file.txt";
    node.host.writeBytes(path, b"hello").unwrap();
    node.workspace().scan().unwrap();
    let time = node.host.modifiedAt(path).unwrap();
    node.host.writeBytes(path, b"world").unwrap();
    node.host.setModifiedAt(path, time + 10).unwrap();
    let reads = node.reads();
    assert_eq!(node.workspace().scan().unwrap(), 1);
    assert_eq!(node.reads() - reads, 1);
    assert_eq!(node.workspace().scan().unwrap(), 0);
}

/// Retaining both metadata fields declares equality under the file contract.
#[test]
fn equal_time_and_size_do_not_read_or_hash_contents() {
    let node = CountedNode::new();
    let path = "workspaces/project/file.txt";
    node.host.writeBytes(path, b"hello").unwrap();
    node.workspace().scan().unwrap();
    let time = node.host.modifiedAt(path).unwrap();
    node.host.writeBytes(path, b"world").unwrap();
    node.host.setModifiedAt(path, time).unwrap();
    let reads = node.reads();
    assert_eq!(node.workspace().scan().unwrap(), 0);
    assert_eq!(node.reads(), reads);
}

#[test]
fn structured_same_size_writes_publish_on_discovery() {
    let a = Node::new();
    let b = Node::new();
    a.write("workspaces/project/file.txt", b"hello");
    a.workspace().scan().unwrap();
    RuntimeFileSyncStore::new(a.host.clone(), SYNC)
        .writeBytes("workspaces/project/file.txt", b"world")
        .unwrap();
    assert!(a.host.exists("runtime/sync/workspace_dirty.json").unwrap());
    a.workspace().scan().unwrap();
    assert!(!a.host.exists("runtime/sync/workspace_dirty.json").unwrap());
    a.send(&b);
    assert_eq!(
        b.host.readBytes("workspaces/project/file.txt").unwrap(),
        b"world"
    );
}


#[test]
fn editable_policy_gitignore_nested_negation_and_build_pruning() {
    let a = Node::new();
    a.write("workspaces/p/.gitignore", b"*.cache\n!keep.cache\n.operit/\n");
    a.write("workspaces/p/build/generated", b"never upload");
    a.write("workspaces/p/remove.cache", b"ignore");
    a.write("workspaces/p/keep.cache", b"keep");
    a.write("workspaces/p/sub/.gitignore", b"*.txt\n!important.txt\n");
    a.write("workspaces/p/sub/no.txt", b"ignore");
    a.write("workspaces/p/sub/important.txt", b"keep");
    operit_store::FileSyncPolicy::FileSyncPolicy::ensure(a.host.as_ref(), "workspaces/p").unwrap();
    a.workspace().scan().unwrap();
    let paths: Vec<_> = a.ops().into_iter().map(|o| o.entityId).collect();
    assert!(paths.contains(&"workspaces/p/.operit/sync.json".into()));
    assert!(paths.contains(&"workspaces/p/keep.cache".into()));
    assert!(paths.contains(&"workspaces/p/sub/important.txt".into()));
    assert!(!paths.iter().any(|p| p.contains("generated") || p.ends_with("remove.cache") || p.ends_with("no.txt")));
}

/// Exclusions must prune whole trees before traversal and locator generation, even with a tiny byte limit.
#[test]
fn excluded_trees_are_not_traversed_or_replaced_by_many_locators() {
    let node = CountedNode::new();
    let mut policy = operit_store::FileSyncPolicy::FileSyncPolicy::default();
    policy.maxFileBytes = 1;
    policy.exclude.push("bulk-input/".into());
    node.host.writeBytes("workspaces/project/.operit/sync.json", &serde_json::to_vec(&policy).unwrap()).unwrap();
    for directory in ["node_modules", "build", "bulk-input"] {
        for index in 0..12 {
            node.host.writeBytes(&format!("workspaces/project/{directory}/nested/{index}.bin"), b"large enough to need a locator if traversed").unwrap();
        }
    }
    node.host.writeBytes("workspaces/project/report.md", b"x").unwrap();
    node.workspace().scan().unwrap();
    let listed = node.host.directoryListings.lock().unwrap();
    let operations = node._node.ops();
    for directory in ["node_modules", "build", "bulk-input"] {
        let prefix = format!("workspaces/project/{directory}");
        assert!(!listed.iter().any(|path| path.starts_with(&prefix)), "excluded tree was traversed: {prefix}");
        assert!(!operations.iter().any(|operation| operation.entityId.starts_with(&prefix)), "excluded tree created files or locators: {prefix}");
    }
    assert!(operations.iter().any(|operation| operation.entityId == "workspaces/project/report.md"));
}

#[test]
fn adding_ignore_rules_does_not_delete_peer_files_and_invalid_policy_is_safe() {
    let a = Node::new(); let b = Node::new();
    a.write("workspaces/p/a.txt", b"keep both copies");
    a.workspace().scan().unwrap(); a.send(&b);
    a.write("workspaces/p/.operit/sync.json", br#"{"exclude":["*.txt"]}"#);
    a.workspace().scan().unwrap(); a.send(&b);
    assert_eq!(b.host.readBytes("workspaces/p/a.txt").unwrap(), b"keep both copies");
    assert!(!a.ops().iter().any(|o| o.operation == "delete"));
    let count = a.ops().len();
    a.write("workspaces/p/.operit/sync.json", b"{ invalid");
    a.write("workspaces/healthy/next.txt", b"keep scanning");
    assert_eq!(a.workspace().scan().unwrap(), 1);
    assert_eq!(a.ops().len(), count + 1);
    assert!(!a.ops().iter().any(|o| o.operation == "delete"));
}

#[test]
fn oversize_file_syncs_locator_without_uploading_original_and_can_shrink() {
    let a = Node::new(); let b = Node::new();
    operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(a.host.clone()).initialize().unwrap();
    a.write("workspaces/p/.operit/sync.json", br#"{"maxFileBytes":4}"#);
    a.write("workspaces/p/big.bin", b"large source stays here");
    a.workspace().scan().unwrap(); a.send(&b);
    assert!(!b.host.exists("workspaces/p/big.bin").unwrap());
    let pointer = b.host.readBytes("workspaces/p/big.bin.operit-ref.json").unwrap();
    let reference: operit_store::FileSyncPolicy::FileReference = serde_json::from_slice(&pointer).unwrap();
    assert_eq!(reference.path, "/operitapp/workspaces/p/big.bin");
    assert_eq!(reference.fileSize, 23);
    assert!(!a.ops().iter().any(|o| o.entityId == "workspaces/p/big.bin"));
    a.write("workspaces/p/big.bin", b"tiny");
    a.workspace().scan().unwrap(); a.send(&b);
    assert_eq!(b.host.readBytes("workspaces/p/big.bin").unwrap(), b"tiny");
    assert!(!b.host.exists("workspaces/p/big.bin.operit-ref.json").unwrap());
}

#[test]
fn total_budget_and_output_and_attachment_roots_use_same_policy() {
    let a = Node::new(); let b = Node::new();
    operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(a.host.clone()).initialize().unwrap();
    a.write("runtime/data/chat_outputs/2026-11-04/.operit/sync.json", br#"{"maxTotalBytes":4}"#);
    a.write("runtime/data/chat_outputs/2026-11-04/a.md", b"one");
    a.write("runtime/data/chat_outputs/2026-11-04/b.md", b"two");
    a.write("runtime/data/chat_attachments/chat/file.txt", b"small attachment");
    a.workspace().scan().unwrap(); a.send(&b);
    assert_eq!(b.host.readBytes("runtime/data/chat_outputs/2026-11-04/a.md").unwrap(), b"one");
    assert!(!b.host.exists("runtime/data/chat_outputs/2026-11-04/b.md").unwrap());
    assert!(b.host.exists("runtime/data/chat_outputs/2026-11-04/b.md.operit-ref.json").unwrap());
    assert_eq!(b.host.readBytes("runtime/data/chat_attachments/chat/file.txt").unwrap(), b"small attachment");
    assert_eq!(b.workspace().scan().unwrap(), 0);
}

#[test]
fn ignored_incoming_file_is_restored_when_rule_is_removed() {
    let a = Node::new(); let b = Node::new();
    b.write("workspaces/p/.operit/sync.json", br#"{"exclude":["*.txt"]}"#);
    a.write("workspaces/p/a.txt", b"arrived while ignored");
    a.workspace().scan().unwrap(); a.send(&b);
    assert!(!b.host.exists("workspaces/p/a.txt").unwrap());
    b.write("workspaces/p/.operit/sync.json", b"{}");
    b.workspace().scan().unwrap();
    assert_eq!(b.host.readBytes("workspaces/p/a.txt").unwrap(), b"arrived while ignored");
}

#[test]
fn invalid_policy_defers_incoming_file_without_blocking_other_workspaces() {
    let a = Node::new();
    let b = Node::new();
    b.write("workspaces/p/.operit/sync.json", b"{ invalid");
    a.write("workspaces/p/incoming.txt", b"arrived while policy was broken");
    a.workspace().scan().unwrap();
    a.send(&b);
    assert!(!b.host.exists("workspaces/p/incoming.txt").unwrap());

    b.write("workspaces/healthy/note.txt", b"independent local edit");
    assert_eq!(b.workspace().scan().unwrap(), 1);
    b.write("workspaces/p/.operit/sync.json", b"{}");
    b.workspace().scan().unwrap();
    assert_eq!(b.host.readBytes("workspaces/p/incoming.txt").unwrap(), b"arrived while policy was broken");
}

/// Bindings and their relative files use the same existing Space file protocol.
#[test]
fn output_bindings_and_files_replicate_through_runtime_file_sync() {
    use operit_store::ChatOutputStore::ChatOutputStore;
    let a = Node::new();
    let b = Node::new();
    let outputs = ChatOutputStore::new(a.host.clone());
    let base = outputs.bind("parent", 1793740800123, None).unwrap();
    outputs.bind("child", 1793827200123, Some(&base)).unwrap();
    outputs.prepare("parent").unwrap();
    let root = ChatOutputStore::storagePath(&base).unwrap();
    a.write(&format!("{root}/report.md"), b"![chart](images/chart.png)");
    a.write(&format!("{root}/images/chart.png"), &[0, 255, 17]);
    a.write(
        &format!("{root}/.operit/attachments/input/note.txt"),
        b"shared input",
    );
    a.write(
        "runtime/data/attachment_sources/local/large.bin",
        b"local source",
    );
    a.workspace().scan().unwrap();
    a.send(&b);

    let peer_outputs = ChatOutputStore::new(b.host.clone());
    assert_eq!(
        peer_outputs.directory("parent").unwrap(),
        Some(base.clone())
    );
    assert_eq!(peer_outputs.directory("child").unwrap(), Some(base.clone()));
    assert_eq!(
        peer_outputs.bind("parent", 1794604800123, None).unwrap(),
        base
    );
    assert_eq!(
        b.host.readBytes(&format!("{root}/report.md")).unwrap(),
        b"![chart](images/chart.png)"
    );
    assert_eq!(
        b.host
            .readBytes(&format!("{root}/images/chart.png"))
            .unwrap(),
        [0, 255, 17]
    );
    assert_eq!(
        b.host
            .readBytes(&format!("{root}/.operit/attachments/input/note.txt"))
            .unwrap(),
        b"shared input"
    );
    assert!(!b
        .host
        .exists("runtime/data/attachment_sources/local/large.bin")
        .unwrap());
    assert_eq!(peer_outputs.prepare("child").unwrap(), Some(base.clone()));
    assert_eq!(b.workspace().scan().unwrap(), 0);
    // Opening a new store after restart retains the received binding.
    assert_eq!(
        ChatOutputStore::new(b.host.clone())
            .directory("child")
            .unwrap(),
        Some(base)
    );
}

/// A source removed between listing and stat must not block unrelated files or publish a guessed deletion.
#[test]
fn disappearing_file_preserves_inventory_until_the_next_scan() {
    let a = CountedNode::new();
    let b = Node::new();
    let path = "workspaces/project/disappearing.txt";
    a.host.writeBytes(path, b"original").unwrap();
    assert_eq!(a.workspace().scan().unwrap(), 1);
    a._node.send(&b);
    a.host.writeBytes("workspaces/project/healthy.txt", b"still syncing").unwrap();
    *a.host.removeBeforeMetadata.lock().unwrap() = Some(path.into());
    assert_eq!(a.workspace().scan().unwrap(), 1);
    a._node.send(&b);
    assert_eq!(b.host.readBytes(path).unwrap(), b"original");
    assert_eq!(b.host.readBytes("workspaces/project/healthy.txt").unwrap(), b"still syncing");
    assert_eq!(a.workspace().scan().unwrap(), 1);
    a._node.send(&b);
    assert!(!b.host.exists(path).unwrap());
    assert_eq!(a.workspace().scan().unwrap(), 0);
}

/// systemd ships literal backslashes on POSIX; they must never alias slash-separated storage paths.
#[cfg(unix)]
#[test]
fn systemd_escaped_filenames_do_not_block_workspace_sync_or_alias_directories() {
    use std::os::unix::fs::symlink;
    let a = Node::new();
    let b = Node::new();
    let directory = a.root.join("workspaces/project/usr/lib/systemd/system");
    std::fs::create_dir_all(&directory).unwrap();
    let escaped = directory.join(r"system-systemd\x2dcryptsetup.slice");
    std::fs::write(&escaped, b"[Slice]").unwrap();
    symlink("missing.service", directory.join("broken.service")).unwrap();
    a.write("workspaces/project/notes.txt", b"normal workspace file");
    assert_eq!(a.workspace().scan().unwrap(), 1);
    assert_eq!(a.workspace().scan().unwrap(), 0);
    a.send(&b);
    assert_eq!(b.host.readBytes("workspaces/project/notes.txt").unwrap(), b"normal workspace file");
    assert!(!b.host.exists("workspaces/project/usr/lib/systemd/system/system-systemd/x2dcryptsetup.slice").unwrap());
    assert_eq!(std::fs::read(&escaped).unwrap(), b"[Slice]");
}

// Models originals written by older peers, including blobs that were never downloaded.
fn legacy_file_operation(node: &Node, path: &str, kind: &str) -> SyncOperation {
    use operit_store::SyncOperationStore::{NewSyncOperation, SyncOperationSemantics};
    node.log().appendLocalOperation("legacy-peer", NewSyncOperation {
        domain: RUNTIME_FILE_SYNC_DOMAIN.into(), entityType: "file".into(),
        entityId: path.into(), operation: kind.into(), semantics: SyncOperationSemantics::EntityState,
        payload: if kind == "upsert" { serde_json::json!({"contentHash": "a".repeat(64), "size": 7}) } else { serde_json::Value::Null },
    }).unwrap()
}

/// Scanning retains incompatible source files locally and synchronizes portable names.
#[cfg(unix)]
#[test]
fn incompatible_names_stay_local_while_normal_files_sync() {
    let a = Node::new();
    let b = Node::new();
    let excluded = [
        "workspaces/p/query??&MsgID=123.jpg", "workspaces/p/CON.txt",
        "workspaces/p/trailing.", "workspaces/p/trailing ", "workspaces/p/a:b.txt",
        "workspaces/p/nested?/ok.txt", "workspaces/p/a\\b.txt",
        "workspaces/CON/.operit/sync.json",
        "runtime/data/chat_outputs/chat/.operit/attachments/id/image?.jpg",
        "runtime/data/chat_attachments/chat/id/NUL.jpg",
        "runtime/data/chat_outputs/chat/...",
        "workspaces/p/..../nested.txt",
    ];
    for path in excluded { a.write(path, b"keep local"); }
    let allowed = ["workspaces/p/中文 图片.jpg", "runtime/data/chat_outputs/chat/good.txt", "runtime/data/chat_attachments/chat/id/good.jpg"];
    for path in allowed { a.write(path, b"sync"); }
    assert_eq!(a.workspace().scan().unwrap(), allowed.len());
    assert_eq!(a.ops().len(), allowed.len());
    a.send(&b);
    for path in excluded {
        assert_eq!(a.host.readBytes(path).unwrap(), b"keep local");
        assert!(!b.host.exists(path).unwrap(), "{path}");
    }
    for path in allowed { assert_eq!(b.host.readBytes(path).unwrap(), b"sync"); }
    assert_eq!(a.workspace().scan().unwrap(), 0);
    assert_eq!(b.workspace().scan().unwrap(), 0);
}

#[test]
fn excluded_inventory_and_recovery_state_are_discarded_without_blobs_or_deletions() {
    let node = Node::new();
    let path = "runtime/data/chat_outputs/chat/.operit/attachments/id/image?.jpg";
    let op = legacy_file_operation(&node, path, "upsert");
    let mut old = serde_json::json!({"files": {}, "references": {}, "deferred": {}, "pending": [path, op.payload]});
    old["files"][path] = serde_json::json!({"size": 7, "modifiedAt": 1});
    old["references"][path] = serde_json::json!({"size": 7, "modifiedAt": 1});
    old["deferred"][path] = op.payload.clone();
    node.write("runtime/sync/workspace_inventory.json", &serde_json::to_vec(&old).unwrap());
    node.write("workspaces/p/good.txt", b"normal");
    assert_eq!(node.workspace().scan().unwrap(), 1);
    let ops = node.ops();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].entityId, "workspaces/p/good.txt");
    assert_eq!(ops[0].operation, "upsert");
    let state: serde_json::Value = serde_json::from_slice(&node.host.readBytes("runtime/sync/workspace_inventory.json").unwrap()).unwrap();
    assert!(state["files"].get(path).is_none());
    assert!(state["references"].get(path).is_none());
    assert!(state["deferred"].as_object().unwrap().is_empty());
    assert!(state["pending"].is_null());
    assert_eq!(node.workspace().scan().unwrap(), 0);
}

/// Incoming excluded names never require blobs, materialization, or further replication.
#[test]
fn incoming_incompatible_files_skip_transfer_materialization_and_relay() {
    let old = Node::new();
    let target = Node::new();
    let files = RuntimeFileSyncStore::new(target.host.clone(), SYNC);
    for path in ["workspaces/p/image?.jpg", "workspaces/CON/file.txt", "workspaces/p/nul.jpg", "workspaces/p/trailing.", "workspaces/p/.", "workspaces/p/..", "runtime/data/chat_outputs/chat/...", "workspaces/p/..../nested.txt"] {
        for kind in ["upsert", "delete"] {
            let op = legacy_file_operation(&old, path, kind);
            assert!(RuntimeFileSyncStore::requiredBlob(&op).unwrap().is_none());
            assert!(!target.workspace().wantsBlob(&op).unwrap());
            assert!(!target.workspace().applyOperation(&op, false).unwrap());
            assert!(target.log().applicationOperations().unwrap().is_empty());
            assert!(target.ops().is_empty());
            // Large legacy envelopes must also skip before reading their absent blob.
            let mut envelope = op.clone();
            envelope.payload = serde_json::json!({"transactionBlob": {"contentHash": "b".repeat(64), "size": 40000}});
            assert!(RuntimeFileSyncStore::requiredBlob(&envelope).unwrap().is_none());
            assert_eq!(files.hydrateTransaction(envelope.clone()).unwrap(), envelope);
        }
    }
    old.write("workspaces/p/good.txt", b"normal");
    old.workspace().scan().unwrap();
    old.send(&target);
    assert_eq!(target.host.readBytes("workspaces/p/good.txt").unwrap(), b"normal");
}

#[cfg(unix)]
#[test]
fn excluded_incoming_updates_and_deletes_leave_existing_local_file_untouched() {
    let source = Node::new();
    let target = Node::new();
    let path = "workspaces/p/image?.jpg";
    target.write(path, b"original");
    for kind in ["upsert", "delete"] {
        let op = legacy_file_operation(&source, path, kind);
        target.workspace().applyOperation(&op, false).unwrap();
        assert_eq!(target.host.readBytes(path).unwrap(), b"original");
    }
    assert!(target.ops().is_empty());
}

/// Excluded legacy records are retired even when their transport blobs are unavailable.
#[test]
fn pending_excluded_originals_are_retired_and_cannot_be_requeued() {
    for path in ["workspaces/p/old?.jpg", "workspaces/p/.", "workspaces/p/..", "runtime/data/chat_outputs/chat/...", "workspaces/p/..../nested.txt"] {
        let source = Node::new();
        let target = Node::new();
        let excluded = legacy_file_operation(&source, path, "upsert");
        let normal = legacy_file_operation(&source, "workspaces/p/good.txt", "upsert");
        let mut envelope = excluded.clone();
        let reference = RuntimeFileSyncStore::new(source.host.clone(), SYNC).storeJsonBlob(&excluded).unwrap();
        envelope.payload = serde_json::json!({"transactionBlob": reference});
        // The receiver intentionally never downloads the envelope blob.
        target.log().receiveOperations(&[envelope.clone(), normal.clone()]).unwrap();
        assert_eq!(target.log().pendingOperationCount().unwrap(), 2);
        target.workspace().scan().unwrap();
        assert_eq!(target.log().pendingOperationIds().unwrap(), vec![normal.opId.clone()]);
        assert_eq!(target.log().applicationOperations().unwrap(), vec![normal.clone()]);
        target.log().receiveOperations(&[excluded.clone(), envelope]).unwrap();
        assert_eq!(target.log().pendingOperationIds().unwrap(), vec![normal.opId]);
        assert!(target.ops().iter().all(|op| op.opId != excluded.opId));
    }
}


#[test]
fn batch_inventory_uses_two_checkpoints_and_recovers_after_final_publication_failure() {
    let source = Node::new();
    for i in 0..8 { source.write(&format!("workspaces/batch/{i}.txt"), format!("data-{i}").as_bytes()); }
    source.workspace().scan().unwrap();
    let operations = source.ops();
    let target = CountedNode::new();
    let files = RuntimeFileSyncStore::new(target.host.clone(), SYNC);
    for op in &operations {
        let reference = RuntimeFileSyncStore::requiredBlob(op).unwrap().unwrap();
        files.writeBlob(&reference, &RuntimeFileSyncStore::new(source.host.clone(), SYNC).readBlob(&reference).unwrap()).unwrap();
    }
    target._node.log().receiveOperations(&operations).unwrap();
    target.host.failInventoryAt.store(2, Ordering::SeqCst);
    assert!(target.workspace().materializeOperations(&operations.iter().collect::<Vec<_>>(), false).unwrap_err().contains("injected"));
    assert_eq!(target.host.inventoryWrites.load(Ordering::SeqCst), 2);
    let saved: serde_json::Value = serde_json::from_slice(&target.host.readBytes("runtime/sync/workspace_inventory.json").unwrap()).unwrap();
    assert_eq!(saved["pendingBatch"].as_array().unwrap().len(), 8);
    // Reopen through a different host and recover before scanning local edits.
    let reopened = WorkspaceFileSyncStore::new(target._node.host.clone(), SYNC);
    assert_eq!(reopened.scan().unwrap(), 0);
    for i in 0..8 {
        assert_eq!(target.host.readBytes(&format!("workspaces/batch/{i}.txt")).unwrap(), format!("data-{i}").as_bytes());
    }
    let saved: serde_json::Value = serde_json::from_slice(&target.host.readBytes("runtime/sync/workspace_inventory.json").unwrap()).unwrap();
    assert!(saved.get("pendingBatch").is_none());
    assert_eq!(target._node.log().pendingOperations().unwrap().len(), 8);
    let expected = operations.iter().map(|op|op.opId.clone()).collect::<std::collections::BTreeSet<_>>();
    assert!(target._node.ops().iter().all(|op|expected.contains(&op.opId)), "recovery must not publish incoming files as local edits");
}

#[test]
fn batch_flush_failure_keeps_recovery_intent_and_does_not_publish_versions() {
    let source = Node::new();
    for i in 0..3 { source.write(&format!("workspaces/batch/{i}.txt"), b"durable"); }
    source.workspace().scan().unwrap();
    let operations = source.ops();
    let target = CountedNode::new();
    let files = RuntimeFileSyncStore::new(target.host.clone(), SYNC);
    for op in &operations {
        let reference = RuntimeFileSyncStore::requiredBlob(op).unwrap().unwrap();
        files.writeBlob(&reference, &RuntimeFileSyncStore::new(source.host.clone(), SYNC).readBlob(&reference).unwrap()).unwrap();
    }
    target._node.log().receiveOperations(&operations).unwrap();
    target.host.failFlush.store(true, Ordering::SeqCst);
    assert!(target.workspace().materializeOperations(&operations.iter().collect::<Vec<_>>(), false).unwrap_err().contains("flush failure"));
    assert_eq!(target.host.batchFlushes.load(Ordering::SeqCst), 1);
    for op in &operations { assert!(target._node.log().shouldApplyOperation(op).unwrap()); }
    let saved: serde_json::Value = serde_json::from_slice(&target.host.readBytes("runtime/sync/workspace_inventory.json").unwrap()).unwrap();
    assert_eq!(saved["pendingBatch"].as_array().unwrap().len(), 3);
    let reopened = WorkspaceFileSyncStore::new(target._node.host.clone(), SYNC);
    assert_eq!(reopened.scan().unwrap(), 0);
    for op in &operations {
        assert_eq!(target.host.readBytes(&op.entityId).unwrap(), b"durable");
        assert!(!target._node.log().shouldApplyOperation(op).unwrap());
    }
    assert_eq!(target._node.ops().len(), operations.len());
}

fn wall_clock_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Republishes one received operation under a fresh identity with a shifted
/// createdAt, modelling a peer whose wall clock runs fast (or slow).
fn skewed_copy(operation: &SyncOperation, shift_millis: i64) -> SyncOperation {
    let mut skewed = operation.clone();
    skewed.createdAt += shift_millis;
    skewed.sequence += 1;
    skewed.opId = format!("{}:{}", skewed.originDeviceId, skewed.sequence);
    skewed
}

fn local_file_operation(node: &Node, path: &str) -> SyncOperation {
    let device = node.log().localDeviceId().unwrap();
    node.ops()
        .into_iter()
        .find(|op| op.originDeviceId == device && op.entityId == path)
        .expect("local file operation was published")
}

/// A peer ten minutes ahead must not outrank every later local edit forever:
/// observing its operation raises the local hybrid-clock floor instead.
#[test]
fn fast_peer_clock_does_not_outrank_later_local_edits() {
    let a = Node::new();
    let b = Node::new();
    let path = "workspaces/project/skew.txt";
    a.write(path, b"desktop");
    a.workspace().scan().unwrap();
    a.send(&b);
    let skewed = skewed_copy(&a.ops().pop().unwrap(), 600_000);
    a.log().appendOperations(&[skewed.clone()]).unwrap();
    a.deliver(&b, &skewed);
    assert_eq!(b.host.readBytes(path).unwrap(), b"desktop");

    b.write(path, b"phone edit");
    b.workspace().scan().unwrap();
    let local = local_file_operation(&b, path);
    assert!(
        local.createdAt >= skewed.createdAt + 1,
        "later local edit must outrank the fast peer clock: local={} peer={}",
        local.createdAt,
        skewed.createdAt
    );
    b.send(&a);
    assert_eq!(a.host.readBytes(path).unwrap(), b"phone edit");
    assert_eq!(b.host.readBytes(path).unwrap(), b"phone edit");
}

/// The floor is durable journal state, so a restart cannot lose the skew guard.
#[test]
fn hybrid_clock_floor_survives_restart() {
    use operit_store::SyncOperationStore::{NewSyncOperation, SyncOperationSemantics};
    let a = Node::new();
    let b = Node::new();
    let path = "workspaces/project/restart.txt";
    a.write(path, b"desktop");
    a.workspace().scan().unwrap();
    a.send(&b);
    let skewed = skewed_copy(&a.ops().pop().unwrap(), 600_000);
    a.deliver(&b, &skewed);

    let restarted = Arc::new(NativeRuntimeStorageHost::new(
        b.root.join("runtime"),
        b.root.join("workspaces"),
    ));
    let operation = SyncOperationStore::new(restarted, SYNC)
        .appendLocalOperation(
            &b.log().localDeviceId().unwrap(),
            NewSyncOperation {
                domain: RUNTIME_FILE_SYNC_DOMAIN.into(),
                entityType: "file".into(),
                entityId: "workspaces/project/restart-probe.txt".into(),
                operation: "delete".into(),
                semantics: SyncOperationSemantics::EntityState,
                payload: serde_json::Value::Null,
            },
        )
        .unwrap();
    assert!(operation.createdAt >= skewed.createdAt + 1);
}

/// Without a newer peer timestamp, local createdAt keeps tracking the wall
/// clock; an older peer operation neither pulls it down nor pushes it up.
#[test]
fn wall_clock_timestamps_are_unchanged_without_ahead_peer() {
    let a = Node::new();
    let b = Node::new();
    let path = "workspaces/project/normal.txt";
    let before = wall_clock_millis();
    a.write(path, b"one");
    a.workspace().scan().unwrap();
    let first = a.ops().pop().unwrap();
    let after = wall_clock_millis();
    assert!(first.createdAt >= before && first.createdAt <= after + 100);
    a.send(&b);

    let stale = skewed_copy(&first, -600_000);
    b.log().appendOperations(&[stale]).unwrap();
    let before = wall_clock_millis();
    b.write(path, b"two");
    b.workspace().scan().unwrap();
    let local = local_file_operation(&b, path);
    let after = wall_clock_millis();
    assert!(
        local.createdAt >= before && local.createdAt <= after + 100,
        "local createdAt must stay near the wall clock: {} not in [{before}, {}]",
        local.createdAt,
        after + 100
    );
}

#[test]
fn batch_missing_blob_does_not_block_other_files_or_future_scans() {
    let source = Node::new();
    source.write("workspaces/batch/a.txt", b"missing");
    source.write("workspaces/batch/b.txt", b"valid");
    source.workspace().scan().unwrap();
    let operations = source.ops();
    let target = Node::new();
    for op in operations.iter().filter(|op|op.entityId.ends_with("b.txt")) {
        let reference = RuntimeFileSyncStore::requiredBlob(op).unwrap().unwrap();
        RuntimeFileSyncStore::new(target.host.clone(), SYNC).writeBlob(&reference,
            &RuntimeFileSyncStore::new(source.host.clone(), SYNC).readBlob(&reference).unwrap()).unwrap();
    }
    target.log().receiveOperations(&operations).unwrap();
    assert!(target.workspace().materializeOperations(&operations.iter().collect::<Vec<_>>(), false).is_err());
    assert_eq!(target.host.readBytes("workspaces/batch/b.txt").unwrap(), b"valid");
    assert_eq!(target.workspace().scan().unwrap(), 0);
    assert!(target.log().shouldApplyOperation(operations.iter().find(|op|op.entityId.ends_with("a.txt")).unwrap()).unwrap());
    assert!(!target.log().shouldApplyOperation(operations.iter().find(|op|op.entityId.ends_with("b.txt")).unwrap()).unwrap());
}
