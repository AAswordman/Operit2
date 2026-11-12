use super::*;
use operit_edge_sdk::{host::RuntimeStorageHost, runtime::NodeSpaceService::SpaceJoinStatus};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Directory(PathBuf);
impl Directory {
    /// Allocates an isolated private node directory for each test instance.
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "operit-edge-phone-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    /// Removes only this test's already-resolved private directory.
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

/// Failed storage publication must not change either the active or restarted state.
#[test]
fn failed_publication_preserves_committed_namespace() {
    let directory = Directory::new();
    let storage = storage::PhoneStorage::open(directory.0.clone()).unwrap();
    storage.writeBytes("node/identity", b"original").unwrap();
    fs::create_dir(directory.0.join("node.pending")).unwrap();
    assert!(storage
        .writeBytesAtomically("node/identity", b"replacement")
        .is_err());
    assert_eq!(storage.readBytes("node/identity").unwrap(), b"original");
    let restarted = storage::PhoneStorage::open(directory.0.clone()).unwrap();
    assert_eq!(restarted.readBytes("node/identity").unwrap(), b"original");
}

/// Invalid committed storage must fail startup rather than minting a new node identity.
#[test]
fn corrupt_snapshot_is_not_reset_and_keys_remain_virtual() {
    let directory = Directory::new();
    let storage = storage::PhoneStorage::open(directory.0.clone()).unwrap();
    for path in [
        "../identity",
        "/identity",
        "C:/identity",
        "node/../identity",
        "node\\identity",
    ] {
        assert!(storage.writeBytes(path, b"bad").is_err());
    }
    storage.writeBytes("node/identity", b"valid").unwrap();
    assert_eq!(storage.list("node").unwrap().len(), 1);
    fs::write(directory.0.join("node.json"), b"corrupt").unwrap();
    assert!(storage::PhoneStorage::open(directory.0.clone()).is_err());
    assert_eq!(fs::read(directory.0.join("node.json")).unwrap(), b"corrupt");
}

/// Actual TCP pairing must remain separate from admission and survive a restart.
#[tokio::test(flavor = "current_thread")]
async fn real_tcp_pairing_admission_expression_and_restart() {
    let first_directory = Directory::new();
    let second_directory = Directory::new();
    let first = PhoneNode::start(
        first_directory.0.clone(),
        "Old phone".into(),
        "127.0.0.1:0".into(),
    )
    .await
    .unwrap();
    let second = PhoneNode::start(
        second_directory.0.clone(),
        "Controller".into(),
        "127.0.0.1:0".into(),
    )
    .await
    .unwrap();
    let original_id = first.device.node_id.clone();
    let original_space = first.status().unwrap()["spaceId"].clone();
    let paired = first
        .execute(Command::StartPairing {
            address: second.status().unwrap()["address"].as_str().unwrap().into(),
        })
        .await
        .unwrap();
    let pairing_id = paired["pairing"]["pairingId"].as_str().unwrap().to_owned();
    let prompt = second
        .device
        .services
        .peers()
        .pairingPrompts()
        .unwrap()
        .into_iter()
        .find(|p| p.pairingId == pairing_id)
        .unwrap();
    assert!(first
        .execute(Command::FinishPairing {
            id: pairing_id.clone(),
            code: "00000x".into()
        })
        .await
        .is_err());
    first
        .execute(Command::FinishPairing {
            id: pairing_id,
            code: prompt.confirmationCode,
        })
        .await
        .unwrap();
    assert_eq!(first.status().unwrap()["spaceId"], original_space);
    assert_eq!(
        first.status().unwrap()["peers"].as_array().unwrap().len(),
        1
    );
    assert!(first.status().unwrap()["pendingPairing"].is_null());
    let request = first
        .device
        .space
        .requestDeviceSpaceJoin(second.device.node_id.clone())
        .await
        .unwrap();
    assert_eq!(request.status, SpaceJoinStatus::Pending);
    let incoming = second
        .device
        .space
        .incomingDeviceSpaceJoins()
        .await
        .unwrap();
    let approval = incoming
        .iter()
        .find(|r| r.requestId == request.requestId)
        .unwrap();
    second
        .device
        .space
        .decideDeviceSpaceJoin(approval.requestId.clone(), approval.assignmentVersion, true)
        .await
        .unwrap();
    let joined = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let result = first
                .device
                .space
                .refreshDeviceSpaceJoin(request.requestId.clone())
                .await
                .unwrap();
            if result.status == SpaceJoinStatus::Joined {
                break result;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(joined.status, SpaceJoinStatus::Joined);
    assert_eq!(
        first.status().unwrap()["spaceId"],
        second.status().unwrap()["spaceId"]
    );
    let response = first
        .device
        .services
        .peers()
        .call(
            &second.device.node_id,
            operit_edge_sdk::link::RoutedCoreRequest {
                spaceId: first.status().unwrap()["spaceId"].as_str().unwrap().into(),
                originNodeId: first.device.node_id.clone(),
                targetNodeId: second.device.node_id.clone(),
                ttl: 0,
                routeKind: operit_edge_sdk::link::RoutedCoreRequestKind::Target,
                payload: operit_edge_sdk::link::CoreCallRequest::new(
                    "phone-face-test",
                    "edge.robotFace",
                    "setExpression",
                    operit_edge_sdk::link::toCoreValue(RobotFaceExpressionRequest {
                        expression: "speaking".into(),
                    })
                    .unwrap(),
                ),
            },
        )
        .await;
    response.result.unwrap();
    assert_eq!(second.status().unwrap()["expression"], "speaking");
    first
        .execute(Command::Expression {
            expression: "listening".into(),
        })
        .await
        .unwrap();
    assert_eq!(first.status().unwrap()["expression"], "listening");
    assert!(first
        .execute(Command::Expression {
            expression: "unregistered".into()
        })
        .await
        .is_err());
    first.execute(Command::Stop).await.unwrap();
    drop(first);
    let restarted = PhoneNode::start(
        first_directory.0.clone(),
        "Old phone".into(),
        "127.0.0.1:0".into(),
    )
    .await
    .unwrap();
    assert_eq!(restarted.device.node_id, original_id);
    assert_eq!(
        restarted.status().unwrap()["peers"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        restarted.status().unwrap()["spaceId"],
        second.status().unwrap()["spaceId"]
    );
    restarted.execute(Command::Stop).await.unwrap();
    second.execute(Command::Stop).await.unwrap();
}
