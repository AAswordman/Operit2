#![allow(non_snake_case)]

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use operit_host_api::HostManager::HostManager;
use operit_util::stream::Stream::Stream;

tokio::task_local! {
    static RUNTIME_HOST_INTERACTION_ORIGIN: RuntimeHostInteractionRequestOrigin;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Host-side interaction category requested by runtime services.
pub enum RuntimeHostInteractionKind {
    #[serde(rename = "browser_automation")]
    BrowserAutomation,
    #[serde(rename = "web_visit")]
    WebVisit,
    #[serde(rename = "compose_webview_controller")]
    ComposeWebViewController,
    #[serde(rename = "system_capture_screenshot")]
    SystemCaptureScreenshot,
    #[serde(rename = "system_recognize_text")]
    SystemRecognizeText,
    #[serde(rename = "audio_play")]
    AudioPlay,
    #[serde(rename = "music_playback")]
    MusicPlayback,
    #[serde(rename = "bluetooth")]
    Bluetooth,
    #[serde(rename = "tts_synthesis")]
    TtsSynthesis,
    #[serde(rename = "tts_playback")]
    TtsPlayback,
    #[serde(rename = "tool_permission")]
    ToolPermission,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Request envelope sent from runtime code to the owning host or controller.
pub struct RuntimeHostInteractionRequest {
    pub requestId: String,
    pub kind: RuntimeHostInteractionKind,
    pub browserAutomation: Option<RuntimeHostInteractionBrowserAutomationPayload>,
    pub webVisit: Option<RuntimeHostInteractionWebVisitPayload>,
    pub composeWebViewController: Option<RuntimeHostInteractionComposeWebViewControllerPayload>,
    pub systemCaptureScreenshot: Option<RuntimeHostInteractionSystemCaptureScreenshotPayload>,
    pub systemRecognizeText: Option<RuntimeHostInteractionSystemRecognizeTextPayload>,
    pub audioPlay: Option<RuntimeHostInteractionAudioPlayPayload>,
    pub musicPlayback: Option<RuntimeHostInteractionMusicPlaybackPayload>,
    pub bluetooth: Option<RuntimeHostInteractionBluetoothPayload>,
    pub ttsSynthesis: Option<RuntimeHostInteractionTtsSynthesisPayload>,
    pub ttsPlayback: Option<RuntimeHostInteractionTtsPlaybackPayload>,
    pub toolPermission: Option<RuntimeHostInteractionToolPermissionPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Logical origin used to route controller-specific runtime host interactions.
pub enum RuntimeHostInteractionRequestOrigin {
    LocalOwner,
    RemoteSession { sessionId: String, deviceId: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeHostInteractionTarget {
    OwnerHost,
    Controller(RuntimeHostInteractionRequestOrigin),
}

#[derive(Clone, Debug)]
struct RuntimeHostInteractionPending {
    target: RuntimeHostInteractionTarget,
    request: RuntimeHostInteractionRequest,
}

impl RuntimeHostInteractionRequest {
    fn browserAutomation(payload: RuntimeHostInteractionBrowserAutomationPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::BrowserAutomation);
        request.browserAutomation = Some(payload);
        request
    }

    fn webVisit(payload: RuntimeHostInteractionWebVisitPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::WebVisit);
        request.webVisit = Some(payload);
        request
    }

    fn composeWebViewController(
        payload: RuntimeHostInteractionComposeWebViewControllerPayload,
    ) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::ComposeWebViewController);
        request.composeWebViewController = Some(payload);
        request
    }

    fn systemCaptureScreenshot() -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::SystemCaptureScreenshot);
        request.systemCaptureScreenshot = Some(RuntimeHostInteractionSystemCaptureScreenshotPayload {});
        request
    }

    fn systemRecognizeText(payload: RuntimeHostInteractionSystemRecognizeTextPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::SystemRecognizeText);
        request.systemRecognizeText = Some(payload);
        request
    }

    fn audioPlay(payload: RuntimeHostInteractionAudioPlayPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::AudioPlay);
        request.audioPlay = Some(payload);
        request
    }

    fn musicPlayback(payload: RuntimeHostInteractionMusicPlaybackPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::MusicPlayback);
        request.musicPlayback = Some(payload);
        request
    }

    fn bluetooth(payload: RuntimeHostInteractionBluetoothPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::Bluetooth);
        request.bluetooth = Some(payload);
        request
    }

    fn ttsSynthesis(payload: RuntimeHostInteractionTtsSynthesisPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::TtsSynthesis);
        request.ttsSynthesis = Some(payload);
        request
    }

    fn ttsPlayback(payload: RuntimeHostInteractionTtsPlaybackPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::TtsPlayback);
        request.ttsPlayback = Some(payload);
        request
    }

    fn toolPermission(payload: RuntimeHostInteractionToolPermissionPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::ToolPermission);
        request.toolPermission = Some(payload);
        request
    }

    fn empty(kind: RuntimeHostInteractionKind) -> Self {
        Self {
            requestId: Uuid::new_v4().to_string(),
            kind,
            browserAutomation: None,
            webVisit: None,
            composeWebViewController: None,
            systemCaptureScreenshot: None,
            systemRecognizeText: None,
            audioPlay: None,
            musicPlayback: None,
            bluetooth: None,
            ttsSynthesis: None,
            ttsPlayback: None,
            toolPermission: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Browser automation request payload sent to the owner host.
pub struct RuntimeHostInteractionBrowserAutomationPayload {
    pub requestId: String,
    pub toolName: String,
    pub parametersJson: String,
    pub requestedAtMillis: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// HTTP header entry used by a web visit request.
pub struct RuntimeHostInteractionWebVisitHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Web visit request payload sent to the owner host.
pub struct RuntimeHostInteractionWebVisitPayload {
    pub requestId: String,
    pub url: String,
    pub headers: Vec<RuntimeHostInteractionWebVisitHeader>,
    pub userAgent: String,
    pub includeImageLinks: bool,
    pub requestedAtMillis: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Compose WebView controller command payload.
pub struct RuntimeHostInteractionComposeWebViewControllerPayload {
    pub commandJson: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Screenshot capture request marker.
pub struct RuntimeHostInteractionSystemCaptureScreenshotPayload {}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// OCR request payload sent to the owner host.
pub struct RuntimeHostInteractionSystemRecognizeTextPayload {
    pub imagePath: String,
    pub language: String,
    pub quality: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Audio playback request payload.
pub struct RuntimeHostInteractionAudioPlayPayload {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Music playback command payload.
pub struct RuntimeHostInteractionMusicPlaybackPayload {
    pub command: String,
    pub source: Option<String>,
    pub sourceType: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub loopPlayback: bool,
    pub volume: f64,
    pub positionMs: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Music playback response returned by the owner host.
pub struct RuntimeHostInteractionMusicPlaybackResponse {
    pub state: String,
    pub source: Option<String>,
    pub sourceType: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub durationMs: Option<i64>,
    pub positionMs: i64,
    pub bufferedPositionMs: i64,
    pub volume: f64,
    pub loopPlayback: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Bluetooth command payload sent to the owner host.
pub struct RuntimeHostInteractionBluetoothPayload {
    pub command: String,
    pub paramsJson: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Bluetooth command response returned by the owner host.
pub struct RuntimeHostInteractionBluetoothResponse {
    pub resultJson: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// TTS synthesis request payload.
pub struct RuntimeHostInteractionTtsSynthesisPayload {
    pub text: String,
    pub voice: String,
    pub locale: String,
    pub speed: f64,
    pub pitch: f64,
    pub outputFormat: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// TTS playback command payload.
pub struct RuntimeHostInteractionTtsPlaybackPayload {
    pub command: String,
    pub text: String,
    pub voice: String,
    pub locale: String,
    pub speed: f64,
    pub pitch: f64,
    pub interrupt: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Tool parameter included in a host permission request.
pub struct RuntimeHostInteractionToolPermissionToolParameter {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Tool identity and arguments included in a host permission request.
pub struct RuntimeHostInteractionToolPermissionTool {
    pub name: String,
    pub parameters: Vec<RuntimeHostInteractionToolPermissionToolParameter>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Tool permission request payload routed to the controlling runtime.
pub struct RuntimeHostInteractionToolPermissionPayload {
    pub tool: RuntimeHostInteractionToolPermissionTool,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Response envelope returned for one host interaction request.
pub struct RuntimeHostInteractionResponse {
    pub browserAutomation: Option<RuntimeHostInteractionBrowserAutomationResponse>,
    pub webVisit: Option<RuntimeHostInteractionWebVisitResponse>,
    pub composeWebViewController: Option<RuntimeHostInteractionComposeWebViewControllerResponse>,
    pub systemCaptureScreenshot: Option<RuntimeHostInteractionSystemCaptureScreenshotResponse>,
    pub systemRecognizeText: Option<RuntimeHostInteractionSystemRecognizeTextResponse>,
    pub audioPlay: Option<RuntimeHostInteractionAudioPlayResponse>,
    pub musicPlayback: Option<RuntimeHostInteractionMusicPlaybackResponse>,
    pub bluetooth: Option<RuntimeHostInteractionBluetoothResponse>,
    pub ttsSynthesis: Option<RuntimeHostInteractionTtsSynthesisResponse>,
    pub ttsPlayback: Option<RuntimeHostInteractionTtsPlaybackResponse>,
    pub toolPermission: Option<RuntimeHostInteractionToolPermissionResponse>,
}

impl RuntimeHostInteractionResponse {
    /// Builds a browser automation response envelope.
    pub fn browserAutomation(response: RuntimeHostInteractionBrowserAutomationResponse) -> Self {
        let mut value = Self::empty();
        value.browserAutomation = Some(response);
        value
    }

    /// Builds a web visit response envelope.
    pub fn webVisit(response: RuntimeHostInteractionWebVisitResponse) -> Self {
        let mut value = Self::empty();
        value.webVisit = Some(response);
        value
    }

    /// Builds a Compose WebView controller response envelope.
    pub fn composeWebViewController(
        response: RuntimeHostInteractionComposeWebViewControllerResponse,
    ) -> Self {
        let mut value = Self::empty();
        value.composeWebViewController = Some(response);
        value
    }

    /// Builds a screenshot response envelope.
    pub fn systemCaptureScreenshot(
        response: RuntimeHostInteractionSystemCaptureScreenshotResponse,
    ) -> Self {
        let mut value = Self::empty();
        value.systemCaptureScreenshot = Some(response);
        value
    }

    /// Builds an OCR response envelope.
    pub fn systemRecognizeText(
        response: RuntimeHostInteractionSystemRecognizeTextResponse,
    ) -> Self {
        let mut value = Self::empty();
        value.systemRecognizeText = Some(response);
        value
    }

    /// Builds an audio playback response envelope.
    pub fn audioPlay(response: RuntimeHostInteractionAudioPlayResponse) -> Self {
        let mut value = Self::empty();
        value.audioPlay = Some(response);
        value
    }

    /// Builds a music playback response envelope.
    pub fn musicPlayback(response: RuntimeHostInteractionMusicPlaybackResponse) -> Self {
        let mut value = Self::empty();
        value.musicPlayback = Some(response);
        value
    }

    /// Builds a Bluetooth response envelope.
    pub fn bluetooth(response: RuntimeHostInteractionBluetoothResponse) -> Self {
        let mut value = Self::empty();
        value.bluetooth = Some(response);
        value
    }

    /// Builds a TTS synthesis response envelope.
    pub fn ttsSynthesis(response: RuntimeHostInteractionTtsSynthesisResponse) -> Self {
        let mut value = Self::empty();
        value.ttsSynthesis = Some(response);
        value
    }

    /// Builds a TTS playback response envelope.
    pub fn ttsPlayback(response: RuntimeHostInteractionTtsPlaybackResponse) -> Self {
        let mut value = Self::empty();
        value.ttsPlayback = Some(response);
        value
    }

    /// Builds a tool permission response envelope.
    pub fn toolPermission(response: RuntimeHostInteractionToolPermissionResponse) -> Self {
        let mut value = Self::empty();
        value.toolPermission = Some(response);
        value
    }

    fn empty() -> Self {
        Self {
            browserAutomation: None,
            webVisit: None,
            composeWebViewController: None,
            systemCaptureScreenshot: None,
            systemRecognizeText: None,
            audioPlay: None,
            musicPlayback: None,
            bluetooth: None,
            ttsSynthesis: None,
            ttsPlayback: None,
            toolPermission: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Browser automation response payload.
pub struct RuntimeHostInteractionBrowserAutomationResponse {
    pub requestId: String,
    pub success: bool,
    pub result: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Metadata entry extracted during a web visit.
pub struct RuntimeHostInteractionWebVisitMetadataEntry {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Link extracted during a web visit.
pub struct RuntimeHostInteractionWebVisitLink {
    pub url: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Full web visit result returned by the owner host.
pub struct RuntimeHostInteractionWebVisitResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub metadata: Vec<RuntimeHostInteractionWebVisitMetadataEntry>,
    pub links: Vec<RuntimeHostInteractionWebVisitLink>,
    pub imageLinks: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Web visit response payload.
pub struct RuntimeHostInteractionWebVisitResponse {
    pub requestId: String,
    pub success: bool,
    pub result: Option<RuntimeHostInteractionWebVisitResult>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Compose WebView controller response payload.
pub struct RuntimeHostInteractionComposeWebViewControllerResponse {
    pub result: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Screenshot capture response payload.
pub struct RuntimeHostInteractionSystemCaptureScreenshotResponse {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// OCR response payload.
pub struct RuntimeHostInteractionSystemRecognizeTextResponse {
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Audio playback response payload.
pub struct RuntimeHostInteractionAudioPlayResponse {
    pub path: String,
    pub started: bool,
    pub details: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// TTS synthesis response payload.
pub struct RuntimeHostInteractionTtsSynthesisResponse {
    pub audioPath: String,
    pub details: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// TTS playback response payload.
pub struct RuntimeHostInteractionTtsPlaybackResponse {
    pub path: String,
    pub active: bool,
    pub paused: bool,
    pub details: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Tool permission response payload.
pub struct RuntimeHostInteractionToolPermissionResponse {
    pub result: String,
}

#[derive(Debug, Default)]
struct RuntimeHostInteractionState {
    pending: BTreeMap<String, RuntimeHostInteractionPending>,
    responses: BTreeMap<String, RuntimeHostInteractionResponse>,
}

#[derive(Debug)]
struct RuntimeHostInteractionBroker {
    state: Mutex<RuntimeHostInteractionState>,
    changed: Condvar,
}

impl Default for RuntimeHostInteractionBroker {
    fn default() -> Self {
        Self {
            state: Mutex::new(RuntimeHostInteractionState::default()),
            changed: Condvar::new(),
        }
    }
}

static RUNTIME_HOST_INTERACTIONS: OnceLock<RuntimeHostInteractionBroker> = OnceLock::new();

/// Service facade for publishing and responding to host interaction requests.
pub struct RuntimeHostInteractionService;

#[derive(Clone, Debug)]
/// Blocking stream of pending host interaction requests for selected kinds.
pub struct RuntimeHostInteractionEventStream {
    kinds: Vec<RuntimeHostInteractionKind>,
    controllerOrigin: RuntimeHostInteractionRequestOrigin,
}

impl Stream for RuntimeHostInteractionEventStream {
    type Item = RuntimeHostInteractionRequest;

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        let broker = runtimeHostInteractionBroker();
        let mut delivered = BTreeSet::<String>::new();
        let mut state = broker
            .state
            .lock()
            .expect("host interaction mutex poisoned");
        loop {
            let next = state
                .pending
                .values()
                .find(|pending| {
                    !delivered.contains(&pending.request.requestId) && self.matchesPending(pending)
                })
                .map(|pending| pending.request.clone());
            if let Some(request) = next {
                delivered.insert(request.requestId.clone());
                drop(state);
                collector(request);
                state = broker
                    .state
                    .lock()
                    .expect("host interaction mutex poisoned");
                continue;
            }
            state = broker
                .changed
                .wait(state)
                .expect("host interaction mutex poisoned");
        }
    }
}

impl RuntimeHostInteractionService {
    /// Creates the host interaction service facade.
    pub fn getInstance(_context: &HostManager) -> Self {
        Self
    }

    /// Responds to a pending owner-host interaction request.
    pub fn respondOwnerHostInteraction(
        &self,
        requestId: String,
        response: RuntimeHostInteractionResponse,
    ) -> Result<(), String> {
        runtimeHostInteractionBroker().respond(&requestId, response)
    }

    /// Creates an event stream for owner-host interaction requests of selected kinds.
    pub fn ownerHostInteractionEvents(
        &self,
        kinds: Vec<RuntimeHostInteractionKind>,
    ) -> RuntimeHostInteractionEventStream {
        RuntimeHostInteractionEventStream {
            kinds,
            controllerOrigin: currentRuntimeHostInteractionOrigin(),
        }
    }
}

/// Requests browser automation from the owner host and waits for a response.
pub fn requestOwnerBrowserAutomation(
    payload: RuntimeHostInteractionBrowserAutomationPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionBrowserAutomationResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::browserAutomation(payload),
        timeout,
    )?;
    response
        .browserAutomation
        .ok_or_else(|| "browser automation response payload is missing".to_string())
}

/// Requests a web visit from the owner host and waits for a response.
pub fn requestOwnerWebVisit(
    payload: RuntimeHostInteractionWebVisitPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionWebVisitResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::webVisit(payload),
        timeout,
    )?;
    response
        .webVisit
        .ok_or_else(|| "web visit response payload is missing".to_string())
}

/// Requests a Compose WebView controller action from the owner host.
pub fn requestOwnerComposeWebViewController(
    payload: RuntimeHostInteractionComposeWebViewControllerPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionComposeWebViewControllerResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::composeWebViewController(payload),
        timeout,
    )?;
    response
        .composeWebViewController
        .ok_or_else(|| "compose webview response payload is missing".to_string())
}

/// Requests a screenshot capture from the owner host.
pub fn requestOwnerSystemCaptureScreenshot(
    timeout: Duration,
) -> Result<RuntimeHostInteractionSystemCaptureScreenshotResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::systemCaptureScreenshot(),
        timeout,
    )?;
    response
        .systemCaptureScreenshot
        .ok_or_else(|| "system capture screenshot response payload is missing".to_string())
}

/// Requests OCR for an image from the owner host.
pub fn requestOwnerSystemRecognizeText(
    payload: RuntimeHostInteractionSystemRecognizeTextPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionSystemRecognizeTextResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::systemRecognizeText(payload),
        timeout,
    )?;
    response
        .systemRecognizeText
        .ok_or_else(|| "system recognize text response payload is missing".to_string())
}

/// Requests audio playback from the owner host.
pub fn requestOwnerAudioPlay(
    payload: RuntimeHostInteractionAudioPlayPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionAudioPlayResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::audioPlay(payload),
        timeout,
    )?;
    response
        .audioPlay
        .ok_or_else(|| "audio play response payload is missing".to_string())
}

/// Requests music playback control from the owner host.
pub fn requestOwnerMusicPlayback(
    payload: RuntimeHostInteractionMusicPlaybackPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionMusicPlaybackResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::musicPlayback(payload),
        timeout,
    )?;
    response
        .musicPlayback
        .ok_or_else(|| "music playback response payload is missing".to_string())
}

/// Requests Bluetooth command execution from the owner host.
pub fn requestOwnerBluetooth(
    payload: RuntimeHostInteractionBluetoothPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionBluetoothResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::bluetooth(payload),
        timeout,
    )?;
    response
        .bluetooth
        .ok_or_else(|| "bluetooth response payload is missing".to_string())
}

/// Requests TTS synthesis from the owner host.
pub fn requestOwnerTtsSynthesis(
    payload: RuntimeHostInteractionTtsSynthesisPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionTtsSynthesisResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::ttsSynthesis(payload),
        timeout,
    )?;
    response
        .ttsSynthesis
        .ok_or_else(|| "tts synthesis response payload is missing".to_string())
}

/// Requests TTS playback control from the owner host.
pub fn requestOwnerTtsPlayback(
    payload: RuntimeHostInteractionTtsPlaybackPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionTtsPlaybackResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::ttsPlayback(payload),
        timeout,
    )?;
    response
        .ttsPlayback
        .ok_or_else(|| "tts playback response payload is missing".to_string())
}

/// Requests a tool permission decision from the active controller.
pub fn requestOwnerToolPermission(
    payload: RuntimeHostInteractionToolPermissionPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionToolPermissionResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::Controller(currentRuntimeHostInteractionOrigin()),
        RuntimeHostInteractionRequest::toolPermission(payload),
        timeout,
    )?;
    response
        .toolPermission
        .ok_or_else(|| "tool permission response payload is missing".to_string())
}

/// Runs a future with a task-local host interaction origin.
pub async fn withRuntimeHostInteractionOrigin<F, T>(
    origin: RuntimeHostInteractionRequestOrigin,
    future: F,
) -> T
where
    F: Future<Output = T>,
{
    RUNTIME_HOST_INTERACTION_ORIGIN.scope(origin, future).await
}

fn currentRuntimeHostInteractionOrigin() -> RuntimeHostInteractionRequestOrigin {
    match RUNTIME_HOST_INTERACTION_ORIGIN.try_with(Clone::clone) {
        Ok(origin) => origin,
        Err(_) => RuntimeHostInteractionRequestOrigin::LocalOwner,
    }
}

fn runtimeHostInteractionBroker() -> &'static RuntimeHostInteractionBroker {
    RUNTIME_HOST_INTERACTIONS.get_or_init(RuntimeHostInteractionBroker::default)
}

impl RuntimeHostInteractionEventStream {
    fn matchesPending(&self, pending: &RuntimeHostInteractionPending) -> bool {
        if !self.kinds.iter().any(|kind| kind == &pending.request.kind) {
            return false;
        }
        match pending.request.kind {
            RuntimeHostInteractionKind::ToolPermission => {
                pending.target
                    == RuntimeHostInteractionTarget::Controller(self.controllerOrigin.clone())
            }
            _ => pending.target == RuntimeHostInteractionTarget::OwnerHost,
        }
    }
}

impl RuntimeHostInteractionBroker {
    fn request(
        &self,
        target: RuntimeHostInteractionTarget,
        request: RuntimeHostInteractionRequest,
        timeout: Duration,
    ) -> Result<RuntimeHostInteractionResponse, String> {
        let requestId = request.requestId.clone();
        let startedAt = Instant::now();
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
        state.pending.insert(
            requestId.clone(),
            RuntimeHostInteractionPending { target, request },
        );
        self.changed.notify_all();
        loop {
            if let Some(response) = state.responses.remove(&requestId) {
                state.pending.remove(&requestId);
                self.changed.notify_all();
                return Ok(response);
            }
            let elapsed = startedAt.elapsed();
            if elapsed >= timeout {
                state.pending.remove(&requestId);
                self.changed.notify_all();
                return Err(format!("host interaction timed out: {requestId}"));
            }
            let wait = timeout.saturating_sub(elapsed);
            let (nextState, _) = self
                .changed
                .wait_timeout(state, wait)
                .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
            state = nextState;
        }
    }

    fn respond(
        &self,
        requestId: &str,
        response: RuntimeHostInteractionResponse,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
        if !state.pending.contains_key(requestId) {
            return Err(format!("host interaction request not found: {requestId}"));
        }
        state.responses.insert(requestId.to_string(), response);
        self.changed.notify_all();
        Ok(())
    }
}
