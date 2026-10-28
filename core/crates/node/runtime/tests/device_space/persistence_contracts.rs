use super::*;
use operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore;
use operit_store::SyncOperationStore::{
    SyncClock, SyncOperation, SyncOperationOrder, SyncOperationStore,
};
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

/// Opens the real operation log over a node's isolated Host storage.
fn operationStore(router: &CoreNodeRouter) -> SyncOperationStore {
    SyncOperationStore::new(router.localCore.runtimeStorageHost(), RUNTIME_SYNC_DIR_PATH)
}

/// Opens the real content-addressed file store without a second filesystem implementation.
fn fileStore(router: &CoreNodeRouter) -> RuntimeFileSyncStore {
    RuntimeFileSyncStore::new(router.localCore.runtimeStorageHost(), RUNTIME_SYNC_DIR_PATH)
}

/// Reads file-domain operations without inventing operation payloads or synchronization clocks.
fn fileOperations(router: &CoreNodeRouter) -> Vec<SyncOperation> {
    operationStore(router)
        .operationsSince(&SyncClock::empty(), &["runtime_file".into()], usize::MAX)
        .unwrap()
}

/// Transfers a verified blob through the real store APIs; this is a store contract, not a transport test.
fn transferBlob(source: &CoreNodeRouter, target: &CoreNodeRouter, operation: &SyncOperation) {
    if let Some(reference) = RuntimeFileSyncStore::requiredBlob(operation).unwrap() {
        let bytes = fileStore(source)
            .readBlobChunk(&reference.contentHash, 0, reference.size)
            .unwrap();
        fileStore(target).writeBlob(&reference, &bytes).unwrap();
    }
}

/// Exercises store conflict resolution and materialization in the same order as the application service.
fn materializeFileOperation(
    target: &CoreNodeRouter,
    operation: &SyncOperation,
) -> Result<(), String> {
    let store = operationStore(target);
    if store
        .shouldApplyOperation(operation)
        .map_err(|e| e.to_string())?
    {
        RuntimeFileSyncStore::applySyncedOperation(
            target.localCore.runtimeStorageHost(),
            RUNTIME_SYNC_DIR_PATH,
            &operation.entityId,
            &operation.operation,
            operation.payload.clone(),
        )?;
        store
            .recordAppliedOperation(operation)
            .map_err(|e| e.to_string())?;
    }
    store.appendOperation(operation).map_err(|e| e.to_string())
}

/// Verifies binary bytes and deletion tombstones propagate A-B-C and stale updates cannot resurrect files.
#[tokio::test]
async fn binary_file_updates_and_deletion_converge_over_three_store_hops() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (a, _) = approvalService("files-a");
    let (b, _) = approvalService("files-b");
    let (c, _) = approvalService("files-c");
    let path = "runtime/data/user_assets/chain.bin";
    let bytes = [0, 1, 128, 255, 10, 0, 45];
    fileStore(&a).writeBytes(path, &bytes).unwrap();
    let upsert = fileOperations(&a).pop().unwrap();
    transferBlob(&a, &b, &upsert);
    materializeFileOperation(&b, &upsert).unwrap();
    transferBlob(&b, &c, &upsert);
    materializeFileOperation(&c, &upsert).unwrap();
    for node in [&a, &b, &c] {
        assert_eq!(
            node.localCore.runtimeStorageHost().readBytes(path).unwrap(),
            bytes
        );
    }
    fileStore(&a).delete(path).unwrap();
    let deletion = fileOperations(&a)
        .into_iter()
        .find(|op| op.operation == "delete")
        .unwrap();
    for node in [&b, &c] {
        materializeFileOperation(node, &deletion).unwrap();
        materializeFileOperation(node, &deletion).unwrap();
        materializeFileOperation(node, &upsert).unwrap();
        assert!(!node.localCore.runtimeStorageHost().exists(path).unwrap());
        assert!(!operationStore(node).shouldApplyOperation(&upsert).unwrap());
        assert_eq!(
            operationStore(node)
                .localClock()
                .unwrap()
                .sequenceFor(&deletion.originDeviceId),
            deletion.sequence
        );
    }
}

/// Verifies missing and corrupt blobs never advance materialization or overwrite an existing file.
#[tokio::test]
async fn missing_truncated_and_corrupt_blob_delivery_preserve_file_and_clock() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (a, _) = approvalService("blob-source");
    let (b, _) = approvalService("blob-target");
    let path = "runtime/data/user_assets/verified.bin";
    fileStore(&a).writeBytes(path, b"verified").unwrap();
    let operation = fileOperations(&a).pop().unwrap();
    let reference = RuntimeFileSyncStore::requiredBlob(&operation)
        .unwrap()
        .unwrap();
    let host = b.localCore.runtimeStorageHost();
    host.writeBytes(path, b"existing").unwrap();
    let beforeClock = operationStore(&b).localClock().unwrap();
    assert!(materializeFileOperation(&b, &operation).is_err());
    assert!(fileStore(&b).writeBlob(&reference, b"short").is_err());
    assert!(fileStore(&b).writeBlob(&reference, b"corrupt!").is_err());
    assert!(!fileStore(&b).hasBlob(&reference).unwrap());
    assert_eq!(host.readBytes(path).unwrap(), b"existing");
    assert_eq!(operationStore(&b).localClock().unwrap(), beforeClock);
    assert!(operationStore(&b).shouldApplyOperation(&operation).unwrap());
    transferBlob(&a, &b, &operation);
    materializeFileOperation(&b, &operation).unwrap();
    assert_eq!(host.readBytes(path).unwrap(), b"verified");
}

/// Verifies independent same-time writers converge under both arrival orders and duplicate replay.
#[tokio::test]
async fn concurrent_file_writers_converge_for_every_two_operation_permutation() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (a, _) = approvalService("conflict-source-a");
    let (b, _) = approvalService("conflict-source-b");
    let path = "runtime/data/user_assets/conflict.bin";
    fileStore(&a).writeBytes(path, b"content-a").unwrap();
    fileStore(&b).writeBytes(path, b"content-b").unwrap();
    let mut left = fileOperations(&a).pop().unwrap();
    let mut right = fileOperations(&b).pop().unwrap();
    // Pin event time to exercise the production origin-id tie breaker deterministically.
    left.createdAt = 100;
    right.createdAt = 100;
    let expected =
        if SyncOperationOrder::fromOperation(&left) > SyncOperationOrder::fromOperation(&right) {
            b"content-a".as_slice()
        } else {
            b"content-b".as_slice()
        };
    for order in [[0, 1], [1, 0]] {
        let (target, _) = approvalService(&format!("conflict-target-{}", order[0]));
        transferBlob(&a, &target, &left);
        transferBlob(&b, &target, &right);
        let operations = [&left, &right];
        for index in order.into_iter().chain(order) {
            materializeFileOperation(&target, operations[index]).unwrap();
        }
        assert_eq!(
            target
                .localCore
                .runtimeStorageHost()
                .readBytes(path)
                .unwrap(),
            expected
        );
        assert!(!operationStore(&target).shouldApplyOperation(&left).unwrap());
        assert!(!operationStore(&target)
            .shouldApplyOperation(&right)
            .unwrap());
        assert_eq!(fileOperations(&target).len(), 2);
    }
}

/// Verifies file synchronization cannot write node-local credentials or unregistered paths.
#[tokio::test]
async fn file_sync_rejects_node_local_and_path_escape_entities_without_writes() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (node, _) = approvalService("file-boundary");
    // Reading the export clock explicitly settles fixture preference outboxes;
    // capture the byte-exact baseline after this permitted setup publication.
    let beforeClock = operationStore(&node).localClock().unwrap();
    let before = durableFiles(&node);
    for path in [
        "runtime/link_access/identity.preferences.json",
        "runtime/space/../link_access/secret",
        "../outside.bin",
    ] {
        assert!(
            fileStore(&node).writeBytes(path, b"forbidden").is_err(),
            "accepted {path}"
        );
        assert!(RuntimeFileSyncStore::applySyncedOperation(
            node.localCore.runtimeStorageHost(),
            RUNTIME_SYNC_DIR_PATH,
            path,
            "delete",
            serde_json::Value::Null
        )
        .is_err());
    }
    assert_eq!(durableFiles(&node), before);
    assert_eq!(operationStore(&node).localClock().unwrap(), beforeClock);
}

/// Verifies pending and withdrawn applicants cannot invoke the real persistence protocol on a target Space.
#[tokio::test]
async fn pending_cancelled_and_independent_peers_cannot_read_sync_operations() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("sync-denial");
    let beforeA = durableFiles(&pair.a);
    let beforeB = durableFiles(&pair.b);
    let request = pair.request().await;
    for withdrawn in [false, true] {
        if withdrawn {
            pair.applicant
                .cancelDeviceSpaceJoin(request.requestId.clone())
                .await
                .unwrap();
        }
        for method in [
            crate::PeerSync::PeerSyncMethod::SyncClock,
            crate::PeerSync::PeerSyncMethod::SyncOperationsSince,
        ] {
            let reply = pair
                .a
                .callNode(
                    pair.b.localNodeId(),
                    method.request("denied-sync".into(), CoreValue::Null),
                )
                .await;
            assert!(reply.result.is_err());
        }
    }
    assert_eq!(durableFiles(&pair.a), beforeA);
    assert_eq!(durableFiles(&pair.b), beforeB);
}
