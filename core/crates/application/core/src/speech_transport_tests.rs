//! Real JavaScript host -> configured providers -> authenticated TCP -> native device.
use super::*;
use operit_host_api::*;
use operit_host_native_common::{NativeHttpHost, NativeRuntimeStorageHost};
use operit_link::CoreValue;
use operit_node_runtime::NodeServices::{NodeServices, PeerEndpoint, PeerTransport};
use operit_node_runtime::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
use operit_node_runtime::RuntimePeerService::RuntimePeerService;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[path = "../../../../../apps/esp32/src/device_speech.rs"]
mod device_speech;

#[derive(Default)]
pub(super) struct SpeechHttp {
    pub requests: Mutex<Vec<HttpRequestData>>,
    failStt: AtomicBool,
    failTts: AtomicBool,
}
impl HttpStreamHost for SpeechHttp {
    fn openHttpByteStream(
        &self,
        id: String,
        request: HttpRequestData,
        opened: HttpStreamOpenedCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        NativeHttpHost::new().openHttpByteStream(id, request, opened, chunk, closed)
    }
    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        NativeHttpHost::new().closeHttpByteStream(id)
    }
}
impl HttpHost for SpeechHttp {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        if !request.url.starts_with("https://speech.test/") {
            return NativeHttpHost::new().executeHttpRequest(request);
        }
        let stt = request.url.ends_with("/stt");
        let url = request.url.clone();
        let failed = if stt {
            self.failStt.load(Ordering::Relaxed)
        } else {
            self.failTts.load(Ordering::Relaxed)
        };
        self.requests.lock().unwrap().push(request);
        Ok(HttpResponseData {
            finalUrl: url,
            statusCode: if failed { 503 } else { 200 },
            statusMessage: "OK".into(),
            headers: Vec::new(),
            body: if failed {
                b"speech provider failed".to_vec()
            } else if stt {
                br#"{"text":"actual host recognition"}"#.to_vec()
            } else {
                (0..9000).map(|i| (i % 256) as u8).collect()
            },
        })
    }
    fn downloadFiles(
        &self,
        request: HttpDownloadRequest,
        control: HttpDownloadControl,
        progress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        NativeHttpHost::new().downloadFiles(request, control, progress)
    }
}
fn profile(nodeId: String) -> operit_store::CoreSpaceStore::CoreSpaceDeviceProfile {
    operit_store::CoreSpaceStore::CoreSpaceDeviceProfile {
        nodeId,
        displayName: "Speech device".into(),
        userName: String::new(),
        platform: "esp32".into(),
        model: "test".into(),
        coreVersion: None,
        pairingIncarnation: None,
        updatedAt: 1,
    }
}
fn listener(storage: Arc<dyn RuntimeStorageHost>) {
    PeerStateStore::new(storage)
        .saveHostConfig(&PeerHostConfig {
            bindAddress: "127.0.0.1:0".into(),
            transports: vec![PeerTransport::Tcp],
            discoveryEnabled: false,
            portMode: PeerHostPortMode::Automatic,
            updatedAt: 1,
        })
        .unwrap();
}
pub(super) async fn verify(
    core: &mut CoreApplication,
    handler: &AIToolHandler,
    root: &Path,
    http: Arc<SpeechHttp>,
) {
    let core_storage = core
        .localClient()
        .hostManager()
        .runtimeStorageHost
        .clone()
        .unwrap();
    listener(core_storage);
    let peer = operit_node_runtime::HostRuntimePeerService::HostRuntimePeerService::new(
        Arc::new(core.localClient().hostManager().clone()),
        &core.nodeRouter,
        core.deviceInfo().clone(),
    )
    .unwrap();
    core.installNodeServices(NodeServices::new(peer.clone()))
        .unwrap();
    peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("speech-edge/runtime"),
        root.join("speech-edge/workspaces"),
    ));
    listener(storage.clone());
    let id = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone())
        .initialize()
        .unwrap()
        .nodeId;
    let edge_router = operit_node_edge::PeerRouter::EdgePeerRouter::new(id.clone());
    let host = core
        .localClient()
        .hostManager()
        .clone()
        .withRuntimeStorageHost(storage.clone());
    let edge_peer = operit_node_edge::peer::createPeerService(
        Arc::new(host.clone()),
        &edge_router,
        core.deviceInfo().clone(),
        operit_node_runtime::HostRuntimePeerService::PeerRuntimeLimits::default(),
    )
    .unwrap();
    let space = operit_node_runtime::NodeSpaceService::NodeSpaceService::new(
        storage.clone(),
        edge_peer.clone(),
    )
    .unwrap();
    space.initialize(profile(id.clone())).unwrap();
    edge_router.installSpace(space.clone()).unwrap();
    let speech = device_speech::DeviceSpeech::default();
    let (output, input) = speech.handlers();
    edge_router
        .install(Arc::new(
            operit_node_edge::EdgeNode::fromHostManager(host)
                .withNodeServices(NodeServices::new(edge_peer.clone()))
                .withStreamHandlers(output, input),
        ))
        .unwrap();
    edge_peer
        .startListening(&[PeerTransport::Tcp])
        .await
        .unwrap();
    let address = PeerStateStore::new(storage)
        .hostConfig()
        .unwrap()
        .unwrap()
        .bindAddress;
    let pairing = peer
        .startPairing(
            PeerEndpoint {
                nodeId: id.clone(),
                address,
            },
            Some(PeerTransport::Tcp),
        )
        .await
        .unwrap();
    let code = edge_peer.pairingPrompts().unwrap()[0]
        .confirmationCode
        .clone();
    peer.finishPairing(&pairing.pairingId, &code).await.unwrap();
    let access = core.accessServices();
    let join = access.requestDeviceSpaceJoin(id.clone()).await.unwrap();
    space
        .decideDeviceSpaceJoin(join.requestId.clone(), join.assignmentVersion, true)
        .await
        .unwrap();
    access.refreshDeviceSpaceJoin(join.requestId).await.unwrap();
    // Both SDK operations resolve the actual configured provider services.
    let tts_manager =
        operit_runtime::data::preferences::TtsConfigManager::TtsConfigManager::getInstance();
    let tts: operit_runtime::data::preferences::TtsConfigManager::TtsConfigManager = tts_manager;
    let config:operit_model::TtsConfig::TtsConfig=serde_json::from_value(serde_json::json!({
        "id":"","name":"Speech TTS","providerType":"HTTP_TTS","endpoint":"https://speech.test/tts","apiKey":"",
        "model":"test","voice":"test","responseFormat":"wav","speed":1.0,"httpMethod":"POST",
        "requestBody":"{\"input\":\"{text}\"}","contentType":"application/json","headers":[],"responsePipeline":[],"createdAt":0,"updatedAt":0})).unwrap();
    let config = tts.createTtsConfig(config).unwrap();
    tts.setCurrentTtsConfigId(&config.id).unwrap();
    let stt = operit_runtime::data::preferences::SttConfigManager::SttConfigManager::getInstance();
    let config=serde_json::from_value(serde_json::json!({"id":"","name":"Speech STT","providerType":"OPENAI_COMPATIBLE",
        "endpoint":"https://speech.test/stt","apiKey":"","model":"test","fileFieldName":"file","modelFieldName":"model",
        "languageFieldName":"language","responseTextJsonPath":"$.text","headers":[],"createdAt":0,"updatedAt":0})).unwrap();
    let config = stt.createSttConfig(config).unwrap();
    stt.setCurrentSttConfigId(&config.id).unwrap();
    let engine = handler
        .runtimeDependencies()
        .js_execution_provider()
        .create_execution_engine(Arc::new(handler.clone()))
        .unwrap();
    let processor = include_str!(
        "../../../../../plugins/packages/examples/device_speech_processor/dist/processor.js"
    );
    let script = format!(
        r#"{processor}
exports.run=async function(params) {{
    const channel=await Tools.device(params.node,{{timeoutMs:1000}}).transport.open({{
        target:'operit.device.speech',method:'results',property:'requests',args:{{session:'native-test'}}}});
    const adapter={{[Symbol.asyncIterator]:()=>channel,
        send:async value=>{{await channel.send(value);if(value.type==='tts.end')await channel.close();}},close:()=>channel.close()}};
    await exports.processSpeech(adapter,{{
        recognize:audio=>Tools.Stt.transcribe({{audio:Array.from(audio),fileName:'capture.wav',contentType:'audio/wav'}}),
        synthesize:text=>Tools.Tts.synthesize({{text}})}});
    return {{completed:true}};
}};"#
    );
    let mut params = std::collections::BTreeMap::new();
    params.insert("node".into(), id.clone().into());
    params.insert("__operit_toolpkg_api_version".into(), "2.0.0".into());
    let execution = async {
        let result = engine
            .execute_script_function_async(
                script,
                "run".into(),
                params,
                Default::default(),
                None,
                false,
                30000,
            )
            .await;

        result
    };
    let device = async {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while speech.snapshot()["ready"] != true {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1250)).await;
        speech.beginRecognition().unwrap();
        speech.appendPcm(vec![0, 128, 255, 127]).unwrap();
        speech.finishRecognition().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while speech.snapshot()["phase"] != 0 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            speech.snapshot()["text"],
            "actual host recognition",
            "{:?}",
            speech.snapshot()
        );
        speech.synthesize("read the device reply".into()).unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if let Some(packet) = speech.takePlayback() {
                    let CoreValue::Map(mut packet) = packet else {
                        panic!("Expected playback map")
                    };
                    if let Some(CoreValue::Bytes(audio)) = packet.remove("audio") {
                        bytes.extend_from_slice(&audio);
                    }
                    if packet.get("type") == Some(&CoreValue::String("tts.end".into())) {
                        break;
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(
            bytes,
            (0..9000).map(|i| (i % 256) as u8).collect::<Vec<_>>()
        );
    };
    let (result, ()) = tokio::join!(execution, device);
    assert!(result.is_ok(), "{result:?}");
    let requests = http.requests.lock().unwrap();
    let stt = requests.iter().find(|r| r.url.ends_with("/stt")).unwrap();
    assert_eq!(&stt.fileParts[0].content[44..], &[0, 128, 255, 127]);
    assert!(requests
        .iter()
        .find(|r| r.url.ends_with("/tts"))
        .unwrap()
        .body
        .windows(21)
        .any(|b| b == b"read the device reply"));
    drop(requests);
    drop(engine);
    verify_package(handler, root, &speech, &id, http).await;
    edge_peer.stop().await.unwrap();
    peer.stop().await.unwrap();
}

async fn verify_package(
    handler: &AIToolHandler,
    root: &Path,
    speech: &device_speech::DeviceSpeech,
    node: &str,
    http: Arc<SpeechHttp>,
) {
    use operit_tools::ToolExecutionManager::{AITool, ToolParameter};
    use std::io::Write;
    let package = root.join("speech-processor.toolpkg");
    let mut archive = zip::ZipWriter::new(std::fs::File::create(&package).unwrap());
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../plugins/packages/examples/device_speech_processor");
    for file in [
        "manifest.json",
        "dist/main.js",
        "dist/processor.js",
        "dist/packages/device_speech_processor.js",
    ] {
        archive
            .start_file(file, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive
            .write_all(&std::fs::read(source.join(file)).unwrap())
            .unwrap();
    }
    archive.finish().unwrap();
    let manager = handler.getOrCreatePackageManager();
    {
        let mut registry = manager.lock().unwrap();
        registry.loadAvailablePackages();
        registry.packageRegistryReadiness().require_ready().unwrap();
        registry
            .addPackageFileFromExternalStorageResult(&package.to_string_lossy())
            .unwrap();
        let enabled = registry.enableToolPkgContainer("com.operit.device_speech_processor");
        assert!(!enabled.contains("not found"), "{enabled}");
    }
    let mut handler = handler.clone();
    let start = handler
        .executeTool(AITool {
            name: "device_speech_processor:start".into(),
            parameters: vec![ToolParameter {
                name: "node_id".into(),
                value: node.into(),
            }],
        })
        .await;
    assert!(start.success, "{:?}", start.error);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while speech.snapshot()["ready"] != true {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    speech.beginRecognition().unwrap();
    speech.appendPcm(vec![0, 128, 255, 127]).unwrap();
    speech.finishRecognition().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while speech.snapshot()["phase"] != 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        speech.snapshot()["text"],
        "actual host recognition",
        "{:?}",
        speech.snapshot()
    );
    speech
        .synthesize("packaged processor playback".into())
        .unwrap();
    let mut audio = Vec::new();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(CoreValue::Map(mut packet)) = speech.takePlayback() {
                if let Some(CoreValue::Bytes(bytes)) = packet.remove("audio") {
                    audio.extend_from_slice(&bytes);
                }
                if packet.get("type") == Some(&CoreValue::String("tts.end".into())) {
                    break;
                }
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(
        audio,
        (0..9000).map(|i| (i % 256) as u8).collect::<Vec<_>>()
    );
    http.failStt.store(true, Ordering::Relaxed);
    speech.beginRecognition().unwrap();
    speech.appendPcm(vec![0, 128, 255, 127]).unwrap();
    speech.finishRecognition().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while speech.snapshot()["phase"] != 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(speech.snapshot()["error"].as_str().unwrap().contains("503"));
    http.failTts.store(true, Ordering::Relaxed);
    speech.synthesize("failed synthesis".into()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while speech.snapshot()["phase"] != 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(speech.snapshot()["error"].as_str().unwrap().contains("503"));
    assert_eq!(
        speech.snapshot()["ready"],
        true,
        "Provider failure must not lose the transport"
    );
    let stop = handler
        .executeTool(AITool {
            name: "device_speech_processor:stop".into(),
            parameters: vec![],
        })
        .await;
    assert!(stop.success, "{:?}", stop.error);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while speech.snapshot()["ready"] == true {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
