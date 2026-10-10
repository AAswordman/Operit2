use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use operit_edge_contract::audio::*;
use operit_host_api::{AudioCaptureHost, AudioCaptureSession, AudioInputDevice, AudioInputFormat};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TestMic {
    blocks: Vec<Vec<u8>>,
    failure: bool,
    repeat: bool,
    drops: Arc<AtomicUsize>,
}
struct TestRecording {
    blocks: std::collections::VecDeque<Vec<u8>>,
    failure: bool,
    repeat: bool,
    drops: Arc<AtomicUsize>,
}
#[async_trait]
impl AudioCaptureHost for TestMic {
    fn listInputs(&self) -> HostResult<Vec<AudioInputDevice>> {
        Ok(vec![AudioInputDevice {
            inputId: "test-mic".into(),
            name: "Simulated microphone".into(),
            formats: vec![AudioInputFormat::default()],
        }])
    }
    async fn openInput(
        &self,
        input: &str,
        format: &AudioInputFormat,
    ) -> HostResult<Box<dyn AudioCaptureSession>> {
        if input != "test-mic" || format != &AudioInputFormat::default() {
            return Err(HostError::new("Unsupported test input"));
        }
        Ok(Box::new(TestRecording {
            blocks: self.blocks.clone().into(),
            failure: self.failure,
            repeat: self.repeat,
            drops: self.drops.clone(),
        }))
    }
}
#[async_trait]
impl AudioCaptureSession for TestRecording {
    async fn read(&mut self) -> HostResult<Option<Vec<u8>>> {
        tokio::task::yield_now().await;
        if self.repeat {
            return Ok(Some(vec![3; 640]));
        }
        if let Some(block) = self.blocks.pop_front() {
            return Ok(Some(block));
        }
        if self.failure {
            Err(HostError::new("microphone overrun"))
        } else {
            Ok(None)
        }
    }
}
impl Drop for TestRecording {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
async fn admit(core: &CoreFixture, edge: &EdgeFixture) {
    start(core, edge).await;
    let node = edge.service.localNodeId();
    pair(&core.peers, &edge.peers, &edge.storage, node.clone()).await;
    assert!(
        edge.peers.spacePushClient().is_none(),
        "pairing alone must not enable microphone upload"
    );
    let request = core.service.requestDeviceSpaceJoin(node).await.unwrap();
    edge.service
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(
        core.service
            .refreshDeviceSpaceJoin(request.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Joined
    );
}
fn microphone(core: &CoreFixture, edge: &EdgeFixture) -> (String, CoreCallRequest) {
    let id = core
        .audio
        .register(
            edge.service.localNodeId(),
            AudioInputFormat::default(),
            60_000,
        )
        .unwrap();
    let request = StartAudioInput {
        streamId: id.clone(),
        receiverNodeId: core.router.localNodeId(),
        inputId: "test-mic".into(),
        format: AudioInputFormat::default(),
        maxDurationMs: 60_000,
    };
    (
        id,
        CoreCallRequest::new(
            uuid::Uuid::new_v4().to_string(),
            EDGE_AUDIO_TARGET,
            "startInput",
            toCoreValue(request).unwrap(),
        ),
    )
}
fn simulated_edge(
    blocks: Vec<Vec<u8>>,
    failure: bool,
    repeat: bool,
    drops: Arc<AtomicUsize>,
) -> EdgeFixture {
    edge_node_with_audio(
        Arc::new(Storage(
            Mutex::default(),
            true,
            Mutex::default(),
            Mutex::default(),
        )),
        Some(Arc::new(TestMic {
            blocks,
            failure,
            repeat,
            drops,
        })),
    )
}
#[tokio::test]
async fn audio_pcm_upload_over_authenticated_tcp_preserves_bytes_and_rejects_replay() {
    let _guard = TEST_LOCK.lock().await;
    let core = core_node();
    let blocks: Vec<Vec<u8>> = (0..100)
        .map(|n| (0..640).map(|i| (i * 17 + n * 31) as u8).collect())
        .collect();
    let expected: Vec<u8> = blocks.iter().flatten().copied().collect();
    let drops = Arc::new(AtomicUsize::new(0));
    let edge = simulated_edge(blocks, false, false, drops.clone());
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        admit(&core, &edge).await;
        let inputs = core
            .router
            .callNode(
                edge.service.localNodeId(),
                CoreCallRequest::new(
                    "inputs",
                    EDGE_AUDIO_TARGET,
                    "listInputs",
                    CoreValue::emptyMap(),
                ),
            )
            .await;
        assert!(inputs.result.is_ok());
        let (id, request) = microphone(&core, &edge);
        let ingress = CorePushRequest::new(
            "audio-target-bypass",
            CORE_AUDIO_INGRESS_TARGET,
            CORE_AUDIO_INGRESS_METHOD,
        )
        .withArgs(
            toCoreValue(AudioIngressOpen {
                streamId: id.clone(),
                format: AudioInputFormat::default(),
            })
            .unwrap(),
        );
        let bypass = edge
            .peers
            .openPush(
                &core.router.localNodeId(),
                RoutedCoreRequest {
                    spaceId: core.service.deviceSpace().unwrap().spaceId,
                    originNodeId: edge.service.localNodeId(),
                    targetNodeId: core.router.localNodeId(),
                    ttl: 4,
                    routeKind: RoutedCoreRequestKind::Target,
                    payload: ingress,
                },
            )
            .await;
        match bypass {
            Err(error) => assert_eq!(error.code, "AUDIO_ORIGIN_DENIED"),
            Ok(_) => panic!("Target Push bypassed the Space audio ingress"),
        }
        let mut wrong_receiver = request.clone();
        if let CoreValue::Map(args) = &mut wrong_receiver.args {
            args.insert(
                "receiverNodeId".into(),
                CoreValue::String(edge.service.localNodeId()),
            );
        }
        assert!(core
            .router
            .callNode(edge.service.localNodeId(), wrong_receiver)
            .await
            .result
            .is_err());
        assert!(core
            .router
            .callNode(edge.service.localNodeId(), request)
            .await
            .result
            .is_ok());
        let mut received = Vec::new();
        let mut sequence = 0;
        loop {
            let block = core.audio.read(&id).await.unwrap();
            if block.pending {
                continue;
            }
            assert_eq!(block.sequence, sequence);
            assert_eq!(block.sampleOffset, received.len() as u64 / 2);
            if block.done {
                assert!(block.error.is_none(), "{:?}", block.error);
                break;
            }
            let bytes = STANDARD.decode(block.dataBase64).unwrap();
            assert_eq!(bytes.len(), block.byteLength as usize);
            received.extend(bytes);
            sequence += 1;
        }
        assert_eq!(received, expected);
        assert_eq!(
            drops.load(Ordering::SeqCst),
            1,
            "capture is released before terminal ACK"
        );
        let push = edge
            .peers
            .spacePushClient()
            .unwrap()
            .openPushTo(
                &core.router.localNodeId(),
                CorePushRequest::new(
                    "replay",
                    CORE_AUDIO_INGRESS_TARGET,
                    CORE_AUDIO_INGRESS_METHOD,
                )
                .withArgs(
                    toCoreValue(AudioIngressOpen {
                        streamId: id.clone(),
                        format: AudioInputFormat::default(),
                    })
                    .unwrap(),
                ),
            )
            .await;
        assert!(
            push.is_err(),
            "a completed stream must not reopen on any channel"
        );
        core.audio.close(&id);
    })
    .await;
    edge.peers.stop().await.unwrap();
    core.peers.stop().await.unwrap();
    result.unwrap();
}
#[tokio::test]
async fn audio_capture_failure_and_early_close_release_microphone_over_tcp() {
    let _guard = TEST_LOCK.lock().await;
    let core = core_node();
    let drops = Arc::new(AtomicUsize::new(0));
    let edge = simulated_edge(vec![vec![5; 640]; 3], true, false, drops.clone());
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        admit(&core, &edge).await;
        let (id, request) = microphone(&core, &edge);
        assert!(core
            .router
            .callNode(edge.service.localNodeId(), request)
            .await
            .result
            .is_ok());
        let mut bytes = 0;
        loop {
            let block = core.audio.read(&id).await.unwrap();
            bytes += block.byteLength;
            if block.done {
                assert!(block.error.unwrap().contains("microphone overrun"));
                break;
            }
        }
        assert_eq!(bytes, 1920);
        core.audio.close(&id);
        // Wait for the prior Push close and reservation removal to finish.
        for _ in 0..100 {
            let (id, request) = microphone(&core, &edge);
            if core
                .router
                .callNode(edge.service.localNodeId(), request)
                .await
                .result
                .is_ok()
            {
                core.audio.close(&id);
                let stop = CoreCallRequest::new(
                    "stop",
                    EDGE_AUDIO_TARGET,
                    "stopInput",
                    toCoreValue(StopAudioInput { streamId: id }).unwrap(),
                );
                assert!(core
                    .router
                    .callNode(edge.service.localNodeId(), stop)
                    .await
                    .result
                    .is_ok());
                break;
            }
            core.audio.close(&id);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        for _ in 0..100 {
            if drops.load(Ordering::SeqCst) == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        // Controls and other calls remain usable after cancelled audio.
        assert!(core
            .router
            .callNode(
                edge.service.localNodeId(),
                CoreCallRequest::new(
                    "inputs-after-close",
                    EDGE_AUDIO_TARGET,
                    "listInputs",
                    CoreValue::emptyMap()
                )
            )
            .await
            .result
            .is_ok());
    })
    .await;
    edge.peers.stop().await.unwrap();
    core.peers.stop().await.unwrap();
    result.unwrap();
}

#[tokio::test]
async fn audio_early_close_with_full_queue_preserves_shared_tcp_and_releases_capture() {
    let _guard = TEST_LOCK.lock().await;
    let core = core_node();
    let drops = Arc::new(AtomicUsize::new(0));
    let edge = simulated_edge(vec![], false, true, drops.clone());
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        admit(&core, &edge).await;
        let (id, request) = microphone(&core, &edge);
        assert!(core
            .router
            .callNode(edge.service.localNodeId(), request)
            .await
            .result
            .is_ok());
        // Let the existing uploader fill its bounded receiver without consuming.
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        core.audio.close(&id);
        let stop = CoreCallRequest::new(
            "stop-full",
            EDGE_AUDIO_TARGET,
            "stopInput",
            toCoreValue(StopAudioInput { streamId: id }).unwrap(),
        );
        assert!(core
            .router
            .callNode(edge.service.localNodeId(), stop)
            .await
            .result
            .is_ok());
        for _ in 0..100 {
            if drops.load(Ordering::SeqCst) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(core
            .router
            .callNode(
                edge.service.localNodeId(),
                CoreCallRequest::new(
                    "inputs-after-full",
                    EDGE_AUDIO_TARGET,
                    "listInputs",
                    CoreValue::emptyMap()
                )
            )
            .await
            .result
            .is_ok());
    })
    .await;
    edge.peers.stop().await.unwrap();
    core.peers.stop().await.unwrap();
    result.unwrap();
}

#[tokio::test]
async fn audio_pairing_revocation_ends_stream_with_error_and_releases_capture() {
    let _guard = TEST_LOCK.lock().await;
    let core = core_node();
    let drops = Arc::new(AtomicUsize::new(0));
    let edge = simulated_edge(vec![], false, true, drops.clone());
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        admit(&core, &edge).await;
        let (id, request) = microphone(&core, &edge);
        assert!(core
            .router
            .callNode(edge.service.localNodeId(), request)
            .await
            .result
            .is_ok());
        assert!(!core.audio.read(&id).await.unwrap().done);
        core.peers
            .removePairedPeer(&edge.service.localNodeId())
            .await
            .unwrap();
        loop {
            let block = core.audio.read(&id).await.unwrap();
            if block.done {
                assert!(block.error.is_some(), "revocation is not clean EOF");
                break;
            }
        }
        for _ in 0..100 {
            if drops.load(Ordering::SeqCst) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        core.audio.close(&id);
    })
    .await;
    edge.peers.stop().await.unwrap();
    core.peers.stop().await.unwrap();
    result.unwrap();
}
