//! Exercises the public speech tool service with real provider synthesis and VFS files.
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

use operit_host_api::HostManager::{setDefaultHttpHost, HostManager};
use operit_host_api::*;
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::TtsConfig::TtsConfig;
use operit_plugin_sdk::js_sdk::{results::TtsSpeakResultData, tts::TtsSpeakRequest};
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use operit_runtime::data::preferences::TtsConfigManager::TtsConfigManager;
use operit_runtime::services::ToolRuntimeSupportService::ToolRuntimeSupportService;
use operit_runtime::services::TtsPlaybackService::TtsPlaybackService;
use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
use operit_tools::runtime_support::ToolRuntimeSupport;

#[derive(Default)]
struct SpeechHttpHost(Mutex<Vec<serde_json::Value>>);

impl HttpStreamHost for SpeechHttpHost {
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("speech synthesis uses buffered HTTP")
    }
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        unreachable!()
    }
}

impl HttpHost for SpeechHttpHost {
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        panic!("speech synthesis uses buffered HTTP")
    }
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        assert_eq!(request.url, "https://speech.test/speak");
        self.0
            .lock()
            .unwrap()
            .push(serde_json::from_slice(&request.body).unwrap());
        Ok(HttpResponseData {
            finalUrl: request.url,
            statusCode: 200,
            statusMessage: "OK".into(),
            headers: vec![("Content-Type".into(), "audio/wav".into())],
            body: b"generated audio".to_vec(),
        })
    }
}

struct PlaybackState {
    status: TtsPlaybackStatus,
    audio: Vec<Vec<u8>>,
    spoken: Vec<TtsPlaybackRequest>,
}

struct PlaybackHost(Mutex<PlaybackState>, Condvar);

impl PlaybackHost {
    fn new() -> Self {
        Self(
            Mutex::new(PlaybackState {
                status: TtsPlaybackStatus {
                    path: String::new(),
                    active: false,
                    paused: false,
                    details: String::new(),
                },
                audio: vec![],
                spoken: vec![],
            }),
            Condvar::new(),
        )
    }
    fn complete(&self) {
        self.0.lock().unwrap().status.active = false;
    }
    fn wait_for_audio(&self, count: usize) {
        let (state, timeout) = self
            .1
            .wait_timeout_while(self.0.lock().unwrap(), Duration::from_secs(3), |state| {
                state.audio.len() < count
            })
            .unwrap();
        assert!(
            !timeout.timed_out(),
            "played {} files, expected {count}",
            state.audio.len()
        );
    }
}

impl TtsPlaybackHost for PlaybackHost {
    fn supportsSystemSpeech(&self) -> bool {
        true
    }
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
        let bytes = std::fs::read(path)?;
        let mut state = self.0.lock().unwrap();
        state.audio.push(bytes);
        state.status.path = path.into();
        state.status.active = true;
        self.1.notify_all();
        Ok(state.status.clone())
    }
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.0.lock().unwrap();
        state.spoken.push(request);
        state.status.path.clear();
        state.status.active = true;
        Ok(state.status.clone())
    }
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.0.lock().unwrap().status.paused = true;
        self.speechState()
    }
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.0.lock().unwrap().status.paused = false;
        self.speechState()
    }
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.complete();
        self.speechState()
    }
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        Ok(self.0.lock().unwrap().status.clone())
    }
}

fn config(provider: &str) -> TtsConfig {
    TtsConfig {
        id: String::new(),
        name: provider.into(),
        providerType: provider.into(),
        endpoint: "https://speech.test/speak".into(),
        apiKey: String::new(),
        model: "test-model".into(),
        voice: "test-voice".into(),
        responseFormat: "wav".into(),
        speed: 1.0,
        httpMethod: "POST".into(),
        contentType: "application/json".into(),
        requestBody: r#"{"text":"{text}","voice":"{voice}","speed":{speed}}"#.into(),
        headers: vec![],
        responsePipeline: vec![],
        createdAt: 0,
        updatedAt: 0,
    }
}

fn speak(
    scheduler: &Arc<NativeHostRuntimeTaskSchedulerHost>,
    support: Arc<dyn ToolRuntimeSupport>,
    request: TtsSpeakRequest,
) -> Result<TtsSpeakResultData, String> {
    let (tx, rx) = mpsc::channel();
    scheduler
        .scheduleHostRuntimeAsyncTask(
            "public-speech-test",
            Box::new(move || {
                Box::pin(async move {
                    let _ = tx.send(support.speakSpeech(request, None).await);
                })
            }),
        )
        .unwrap();
    rx.recv_timeout(Duration::from_secs(5))
        .expect("public speech startup timed out")
}

fn request(text: &str) -> TtsSpeakRequest {
    TtsSpeakRequest {
        waitForCompletion: None,
        playbackId: None,
        text: text.into(),
        configId: None,
        interrupt: None,
        speed: None,
        pitch: None,
    }
}

#[test]
fn public_speech_uses_selected_provider_overrides_and_all_generated_segments() {
    let root = std::env::temp_dir().join(format!("operit-public-speech-{}", uuid::Uuid::new_v4()));
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    setDefaultRuntimeStorageHost(storage.clone());
    operit_store::RuntimeStorageHost::setDefaultRuntimeSqliteHost(storage.clone());
    operit_store::RuntimeStorageHost::setDefaultHostSecretStore(Arc::new(
        operit_host_native_storage::NativeFileSecretStore(root.join("secrets")),
    ));
    let http = Arc::new(SpeechHttpHost::default());
    setDefaultHttpHost(http.clone());
    let playback_host = Arc::new(PlaybackHost::new());
    let scheduler = Arc::new(NativeHostRuntimeTaskSchedulerHost::new());
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(scheduler.clone());
    let mut context = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    context.runtimeStorageHost = Some(storage);
    context.hostRuntimeTaskSchedulerHost = Some(scheduler.clone());
    context.ttsPlaybackHost = Some(playback_host.clone());
    let holder = Arc::new(tokio::sync::Mutex::new(ChatRuntimeHolder::new(Arc::new(
        context.vfs().unwrap(),
    ))));
    let support = ToolRuntimeSupportService::create(context.clone(), holder);
    let controls = TtsPlaybackService::getInstance(&context).unwrap();
    let manager = TtsConfigManager::getInstance();
    let http_config = manager.createTtsConfig(config("HTTP_TTS")).unwrap();
    manager.setCurrentTtsConfigId(&http_config.id).unwrap();

    let mut utterance = request("Hello from a plugin");
    utterance.speed = Some(1.25);
    let result = speak(&scheduler, support.clone(), utterance).unwrap();
    assert!(result.started);
    assert_eq!(result.providerType, "HTTP_TTS");
    assert_eq!(
        http.0.lock().unwrap()[0],
        serde_json::json!({"text":"Hello from a plugin","voice":"test-voice","speed":1.25})
    );
    assert_eq!(
        playback_host.0.lock().unwrap().audio,
        [b"generated audio".to_vec()]
    );
    assert_eq!(manager.getTtsConfig(&http_config.id).unwrap().speed, 1.0);
    controls.stopSpeech().unwrap();

    // Explicit system voices share the public entry point without changing the current selection.
    let system_config = manager.createTtsConfig(config("SYSTEM_TTS")).unwrap();
    let mut utterance = request("System voice");
    utterance.configId = Some(system_config.id);
    utterance.speed = Some(1.1);
    utterance.pitch = Some(1.2);
    let result = speak(&scheduler, support.clone(), utterance).unwrap();
    assert!(result.started);
    assert_eq!(result.providerType, "SYSTEM_TTS");
    let spoken = playback_host.0.lock().unwrap().spoken[0].clone();
    assert_eq!(
        (spoken.speed, spoken.pitch, spoken.voice.as_str()),
        (1.1, 1.2, "test-voice")
    );
    assert_eq!(manager.getCurrentTtsConfigId().unwrap(), http_config.id);
    assert_eq!(http.0.lock().unwrap().len(), 1);
    controls.stopSpeech().unwrap();

    // Existing provider synthesis segments long text; public playback must consume every file.
    let result = speak(&scheduler, support.clone(), request(&"word ".repeat(900))).unwrap();
    assert!(result.started);
    assert_eq!(http.0.lock().unwrap().len(), 3);
    assert_eq!(playback_host.0.lock().unwrap().audio.len(), 2);
    playback_host.complete();
    playback_host.wait_for_audio(3);
    controls.stopSpeech().unwrap();

    let mut utterance = request("Unsupported pitch");
    utterance.pitch = Some(1.2);
    assert!(speak(&scheduler, support, utterance)
        .err()
        .expect("generated audio must reject unsupported pitch")
        .contains("Pitch overrides"));
    assert_eq!(http.0.lock().unwrap().len(), 3);
    std::fs::remove_dir_all(root).unwrap();
}
