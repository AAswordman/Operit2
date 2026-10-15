#![allow(non_snake_case)]

use core::time::Duration;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use operit_host_api::HostManager::{defaultHostRuntimeTaskSchedulerHost, HostManager};
use operit_host_api::TimeUtils::tryCurrentTimeMillisU128;
use operit_host_api::{
    SystemNotificationActivation, SystemNotificationRequest, SystemOperationHost,
};
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
use operit_util::stream::Stream::{CollectFuture, Stream};
use operit_util::ImagePoolManager::ImagePoolManager;
use operit_util::MediaPoolManager::MediaPoolManager;
use tokio::sync::{oneshot, Notify};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Host-side interaction category requested by runtime services.
pub enum RuntimeHostInteractionKind {
    #[serde(rename = "browser_automation")]
    BrowserAutomation,
    #[serde(rename = "browser_session")]
    BrowserSession,
    #[serde(rename = "web_visit")]
    WebVisit,
    #[serde(rename = "compose_webview_controller")]
    ComposeWebViewController,
    #[serde(rename = "compose_file_picker")]
    ComposeFilePicker,
    #[serde(rename = "compose_file_save")]
    ComposeFileSave,
    #[serde(rename = "system_capture_screenshot")]
    SystemCaptureScreenshot,
    #[serde(rename = "system_language_code")]
    SystemLanguageCode,
    #[serde(rename = "system_recognize_text")]
    SystemRecognizeText,
    #[serde(rename = "system_operation")]
    SystemOperation,
    #[serde(rename = "file_open")]
    FileOpen,
    #[serde(rename = "file_share")]
    FileShare,
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
    #[serde(rename = "local_inference")]
    LocalInference,
    #[serde(rename = "app_notification")]
    AppNotification,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Request envelope sent from runtime code to the owning host or controller.
pub struct RuntimeHostInteractionRequest {
    pub requestId: String,
    pub kind: RuntimeHostInteractionKind,
    pub browserAutomation: Option<RuntimeHostInteractionBrowserAutomationPayload>,
    pub browserSession: Option<RuntimeHostInteractionBrowserSessionPayload>,
    pub webVisit: Option<RuntimeHostInteractionWebVisitPayload>,
    pub composeWebViewController: Option<RuntimeHostInteractionComposeWebViewControllerPayload>,
    pub composeFilePicker: Option<RuntimeHostInteractionComposeFilePickerPayload>,
    pub composeFileSave: Option<RuntimeHostInteractionComposeFileSavePayload>,
    pub systemCaptureScreenshot: Option<RuntimeHostInteractionSystemCaptureScreenshotPayload>,
    pub systemLanguageCode: Option<RuntimeHostInteractionSystemLanguageCodePayload>,
    pub systemRecognizeText: Option<RuntimeHostInteractionSystemRecognizeTextPayload>,
    pub systemOperation: Option<RuntimeHostInteractionSystemOperationPayload>,
    pub fileOpen: Option<RuntimeHostInteractionFileOpenPayload>,
    pub fileShare: Option<RuntimeHostInteractionFileSharePayload>,
    pub audioPlay: Option<RuntimeHostInteractionAudioPlayPayload>,
    pub musicPlayback: Option<RuntimeHostInteractionMusicPlaybackPayload>,
    pub bluetooth: Option<RuntimeHostInteractionBluetoothPayload>,
    pub ttsSynthesis: Option<RuntimeHostInteractionTtsSynthesisPayload>,
    pub ttsPlayback: Option<RuntimeHostInteractionTtsPlaybackPayload>,
    pub localInference: Option<RuntimeHostInteractionLocalInferencePayload>,
    pub appNotification: Option<RuntimeHostInteractionAppNotificationPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeHostInteractionTarget {
    OwnerHost,
}

#[derive(Clone, Debug)]
struct RuntimeHostInteractionPending {
    target: RuntimeHostInteractionTarget,
    request: RuntimeHostInteractionRequest,
    sequence: u64,
    awaitsResponse: bool,
}

impl RuntimeHostInteractionRequest {
    fn browserAutomation(payload: RuntimeHostInteractionBrowserAutomationPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::BrowserAutomation);
        request.browserAutomation = Some(payload);
        request
    }

    fn browserSession(payload: RuntimeHostInteractionBrowserSessionPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::BrowserSession);
        request.browserSession = Some(payload);
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

    /// Builds a request for one Compose DSL file-picker interaction.
    fn composeFilePicker(payload: RuntimeHostInteractionComposeFilePickerPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::ComposeFilePicker);
        request.composeFilePicker = Some(payload);
        request
    }

    /// Creates a request for the owner file-save selector.
    fn composeFileSave(payload: RuntimeHostInteractionComposeFileSavePayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::ComposeFileSave);
        request.composeFileSave = Some(payload);
        request
    }

    fn systemCaptureScreenshot() -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::SystemCaptureScreenshot);
        request.systemCaptureScreenshot =
            Some(RuntimeHostInteractionSystemCaptureScreenshotPayload {});
        request
    }

    /// Builds a request for the current owner host language code.
    fn systemLanguageCode() -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::SystemLanguageCode);
        request.systemLanguageCode = Some(RuntimeHostInteractionSystemLanguageCodePayload {});
        request
    }

    fn systemRecognizeText(payload: RuntimeHostInteractionSystemRecognizeTextPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::SystemRecognizeText);
        request.systemRecognizeText = Some(payload);
        request
    }

    /// Builds a request for one owner-host system operation.
    fn systemOperation(payload: RuntimeHostInteractionSystemOperationPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::SystemOperation);
        request.systemOperation = Some(payload);
        request
    }

    fn fileOpen(payload: RuntimeHostInteractionFileOpenPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::FileOpen);
        request.fileOpen = Some(payload);
        request
    }

    fn fileShare(payload: RuntimeHostInteractionFileSharePayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::FileShare);
        request.fileShare = Some(payload);
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

    /// Builds a request for one owner-host local inference operation.
    fn localInference(payload: RuntimeHostInteractionLocalInferencePayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::LocalInference);
        request.localInference = Some(payload);
        request
    }



    /// Builds a non-blocking notification for one application-level event.
    fn appNotification(payload: RuntimeHostInteractionAppNotificationPayload) -> Self {
        let mut request = Self::empty(RuntimeHostInteractionKind::AppNotification);
        request.appNotification = Some(payload);
        request
    }

    fn empty(kind: RuntimeHostInteractionKind) -> Self {
        Self {
            requestId: Uuid::new_v4().to_string(),
            kind,
            browserAutomation: None,
            browserSession: None,
            webVisit: None,
            composeWebViewController: None,
            composeFilePicker: None,
            composeFileSave: None,
            systemCaptureScreenshot: None,
            systemLanguageCode: None,
            systemRecognizeText: None,
            systemOperation: None,
            fileOpen: None,
            fileShare: None,
            audioPlay: None,
            musicPlayback: None,
            bluetooth: None,
            ttsSynthesis: None,
            ttsPlayback: None,
            localInference: None,
            appNotification: None,
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
/// Browser session command payload sent to the owner host.
pub struct RuntimeHostInteractionBrowserSessionPayload {
    pub commandJson: String,
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
/// Compose DSL file-picker request payload.
pub struct RuntimeHostInteractionComposeFilePickerPayload {
    pub requestJson: String,
}

/// Compose DSL file-save request payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeHostInteractionComposeFileSavePayload {
    pub requestJson: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Screenshot capture request marker.
pub struct RuntimeHostInteractionSystemCaptureScreenshotPayload {}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// System language code request marker.
pub struct RuntimeHostInteractionSystemLanguageCodePayload {}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// OCR request payload sent to the owner host.
pub struct RuntimeHostInteractionSystemRecognizeTextPayload {
    pub imagePath: String,
    pub language: String,
    pub quality: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Generic system operation request sent to the owner host.
pub struct RuntimeHostInteractionSystemOperationPayload {
    pub operation: String,
    pub paramsJson: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// File-open request payload sent to the owner host.
pub struct RuntimeHostInteractionFileOpenPayload {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// File-share request payload sent to the owner host.
pub struct RuntimeHostInteractionFileSharePayload {
    pub path: String,
    pub title: String,
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
    #[serde(default)]
    pub audioPath: Option<String>,
    pub text: String,
    pub voice: String,
    pub locale: String,
    pub speed: f64,
    pub pitch: f64,
    pub interrupt: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Local inference command payload sent to the owner host.
pub struct RuntimeHostInteractionLocalInferencePayload {
    pub method: String,
    pub requestJson: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Tool parameter included in a host permission request.
pub struct RuntimeHostInteractionToolPermissionToolParameter {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Tool identity and arguments included in a host permission request.
pub struct RuntimeHostInteractionToolPermissionTool {
    pub name: String,
    pub parameters: Vec<RuntimeHostInteractionToolPermissionToolParameter>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Chat-scoped tool permission request exposed through chat state.
pub struct RuntimeHostInteractionToolPermissionRequest {
    pub requestId: String,
    pub chatId: String,
    pub tool: RuntimeHostInteractionToolPermissionTool,
    pub description: String,
    pub requestedAtMillis: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Response envelope returned for one host interaction request.
pub struct RuntimeHostInteractionResponse {
    pub error: Option<String>,
    pub browserAutomation: Option<RuntimeHostInteractionBrowserAutomationResponse>,
    pub browserSession: Option<RuntimeHostInteractionBrowserSessionResponse>,
    pub webVisit: Option<RuntimeHostInteractionWebVisitResponse>,
    pub composeWebViewController: Option<RuntimeHostInteractionComposeWebViewControllerResponse>,
    pub composeFilePicker: Option<RuntimeHostInteractionComposeFilePickerResponse>,
    pub composeFileSave: Option<RuntimeHostInteractionComposeFileSaveResponse>,
    pub systemCaptureScreenshot: Option<RuntimeHostInteractionSystemCaptureScreenshotResponse>,
    pub systemLanguageCode: Option<RuntimeHostInteractionSystemLanguageCodeResponse>,
    pub systemRecognizeText: Option<RuntimeHostInteractionSystemRecognizeTextResponse>,
    pub systemOperation: Option<RuntimeHostInteractionSystemOperationResponse>,
    pub fileOpen: Option<RuntimeHostInteractionFileOperationResponse>,
    pub fileShare: Option<RuntimeHostInteractionFileOperationResponse>,
    pub audioPlay: Option<RuntimeHostInteractionAudioPlayResponse>,
    pub musicPlayback: Option<RuntimeHostInteractionMusicPlaybackResponse>,
    pub bluetooth: Option<RuntimeHostInteractionBluetoothResponse>,
    pub ttsSynthesis: Option<RuntimeHostInteractionTtsSynthesisResponse>,
    pub ttsPlayback: Option<RuntimeHostInteractionTtsPlaybackResponse>,
    pub localInference: Option<RuntimeHostInteractionLocalInferenceResponse>,
}

impl RuntimeHostInteractionResponse {
    /// Builds a browser automation response envelope.
    pub fn browserAutomation(response: RuntimeHostInteractionBrowserAutomationResponse) -> Self {
        let mut value = Self::empty();
        value.browserAutomation = Some(response);
        value
    }

    /// Builds a browser session response envelope.
    pub fn browserSession(response: RuntimeHostInteractionBrowserSessionResponse) -> Self {
        let mut value = Self::empty();
        value.browserSession = Some(response);
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

    /// Builds a Compose DSL file-picker response envelope.
    pub fn composeFilePicker(response: RuntimeHostInteractionComposeFilePickerResponse) -> Self {
        let mut value = Self::empty();
        value.composeFilePicker = Some(response);
        value
    }

    /// Creates the response for an owner file-save interaction.
    pub fn composeFileSave(response: RuntimeHostInteractionComposeFileSaveResponse) -> Self {
        let mut value = Self::empty();
        value.composeFileSave = Some(response);
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

    /// Builds a system language response envelope.
    pub fn systemLanguageCode(response: RuntimeHostInteractionSystemLanguageCodeResponse) -> Self {
        let mut value = Self::empty();
        value.systemLanguageCode = Some(response);
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

    /// Builds a system operation response envelope.
    pub fn systemOperation(response: RuntimeHostInteractionSystemOperationResponse) -> Self {
        let mut value = Self::empty();
        value.systemOperation = Some(response);
        value
    }

    /// Builds a file-open response envelope.
    pub fn fileOpen(response: RuntimeHostInteractionFileOperationResponse) -> Self {
        let mut value = Self::empty();
        value.fileOpen = Some(response);
        value
    }

    /// Builds a file-share response envelope.
    pub fn fileShare(response: RuntimeHostInteractionFileOperationResponse) -> Self {
        let mut value = Self::empty();
        value.fileShare = Some(response);
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

    /// Builds a local inference response envelope.
    pub fn localInference(response: RuntimeHostInteractionLocalInferenceResponse) -> Self {
        let mut value = Self::empty();
        value.localInference = Some(response);
        value
    }

    fn empty() -> Self {
        Self {
            error: None,
            browserAutomation: None,
            browserSession: None,
            webVisit: None,
            composeWebViewController: None,
            composeFilePicker: None,
            composeFileSave: None,
            systemCaptureScreenshot: None,
            systemLanguageCode: None,
            systemRecognizeText: None,
            systemOperation: None,
            fileOpen: None,
            fileShare: None,
            audioPlay: None,
            musicPlayback: None,
            bluetooth: None,
            ttsSynthesis: None,
            ttsPlayback: None,
            localInference: None,
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
/// Browser session command response payload.
pub struct RuntimeHostInteractionBrowserSessionResponse {
    pub resultJson: String,
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
/// Compose DSL file-picker response payload.
pub struct RuntimeHostInteractionComposeFilePickerResponse {
    pub resultJson: String,
}

/// Compose DSL file-save response payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeHostInteractionComposeFileSaveResponse {
    pub resultJson: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Screenshot capture response payload.
pub struct RuntimeHostInteractionSystemCaptureScreenshotResponse {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// System language response payload.
pub struct RuntimeHostInteractionSystemLanguageCodeResponse {
    pub languageCode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// OCR response payload.
pub struct RuntimeHostInteractionSystemRecognizeTextResponse {
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Generic system operation response returned by the owner host.
pub struct RuntimeHostInteractionSystemOperationResponse {
    pub resultJson: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// File operation response payload.
pub struct RuntimeHostInteractionFileOperationResponse {
    pub success: bool,
    pub error: Option<String>,
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
/// Local inference command response returned by the owner host.
pub struct RuntimeHostInteractionLocalInferenceResponse {
    pub resultJson: String,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
/// Application event that the owning Flutter process may surface as a system notification.
pub struct RuntimeHostInteractionAppNotificationPayload {
    pub notificationType: String,
    pub title: String,
    pub message: String,
    pub chatId: Option<String>,
    pub messageTimestamp: Option<i64>,
}

#[derive(Debug, Default)]
struct RuntimeHostInteractionState {
    pending: BTreeMap<String, RuntimeHostInteractionPending>,
    responses: BTreeMap<String, RuntimeHostInteractionResponse>,
    nextSequence: u64,
}

struct RuntimeHostInteractionBroker {
    state: Mutex<RuntimeHostInteractionState>,
    changed: Condvar,
    asyncChanged: Notify,
}

impl Default for RuntimeHostInteractionBroker {
    fn default() -> Self {
        Self {
            state: Mutex::new(RuntimeHostInteractionState::default()),
            changed: Condvar::new(),
            asyncChanged: Notify::new(),
        }
    }
}

static RUNTIME_HOST_INTERACTIONS: OnceLock<RuntimeHostInteractionBroker> = OnceLock::new();

#[derive(Default)]
struct ChatToolPermissionState {
    pending: BTreeMap<String, RuntimeHostInteractionToolPermissionRequest>,
    decisions: BTreeMap<String, String>,
}

struct ChatToolPermissionBroker {
    state: Mutex<ChatToolPermissionState>,
    changed: Notify,
    requests: MutableStateFlow<Vec<RuntimeHostInteractionToolPermissionRequest>>,
}

impl Default for ChatToolPermissionBroker {
    fn default() -> Self {
        Self {
            state: Mutex::new(ChatToolPermissionState::default()),
            changed: Notify::new(),
            requests: mutableStateFlow(Vec::new()),
        }
    }
}

static CHAT_TOOL_PERMISSIONS: OnceLock<ChatToolPermissionBroker> = OnceLock::new();

/// Returns the process-wide chat tool permission broker.
fn chatToolPermissionBroker() -> &'static ChatToolPermissionBroker {
    CHAT_TOOL_PERMISSIONS.get_or_init(ChatToolPermissionBroker::default)
}

impl ChatToolPermissionBroker {
    /// Publishes all unresolved requests to subscribed chat state flows.
    fn refreshRequests(&self, state: &ChatToolPermissionState) {
        let mut requests = state
            .pending
            .iter()
            .filter(|(requestId, _)| !state.decisions.contains_key(*requestId))
            .map(|(_, request)| request.clone())
            .collect::<Vec<_>>();
        requests.sort_by(|left, right| {
            left.requestedAtMillis
                .cmp(&right.requestedAtMillis)
                .then_with(|| left.requestId.cmp(&right.requestId))
        });
        self.requests.set_value(requests);
    }

    /// Adds one chat-bound request and awaits its decision or deadline.
    async fn request(
        &self,
        chatId: String,
        tool: RuntimeHostInteractionToolPermissionTool,
        description: String,
        timeout: Duration,
    ) -> Result<String, String> {
        let timeoutMillis = u64::try_from(timeout.as_millis())
            .map_err(|_| "tool permission timeout exceeds u64 milliseconds")?;
        let requestId = Uuid::new_v4().to_string();
        let requestedAtMillis = i64::try_from(tryCurrentTimeMillisU128()?)
            .map_err(|_| "tool permission timestamp exceeds i64 milliseconds")?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|error| format!("tool permission mutex poisoned: {error}"))?;
            state.pending.insert(
                requestId.clone(),
                RuntimeHostInteractionToolPermissionRequest {
                    requestId: requestId.clone(),
                    chatId,
                    tool,
                    description,
                    requestedAtMillis,
                },
            );
            self.refreshRequests(&state);
        }
        self.changed.notify_waiters();

        let (timeoutSender, mut timeoutReceiver) = oneshot::channel();
        if let Err(error) = defaultHostRuntimeTaskSchedulerHost()
            .scheduleDelayedHostRuntimeTask(
                "chat-tool-permission-timeout",
                timeoutMillis,
                Box::new(move || {
                    let _ = timeoutSender.send(());
                }),
            )
        {
            let mut state = self
                .state
                .lock()
                .map_err(|lockError| format!("tool permission mutex poisoned: {lockError}"))?;
            state.pending.remove(&requestId);
            self.refreshRequests(&state);
            self.changed.notify_waiters();
            return Err(error.message);
        }

        loop {
            let changed = self.changed.notified();
            {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|error| format!("tool permission mutex poisoned: {error}"))?;
                if let Some(result) = state.decisions.remove(&requestId) {
                    state.pending.remove(&requestId);
                    self.refreshRequests(&state);
                    return Ok(result);
                }
            }
            tokio::select! {
                _ = changed => {}
                timeout = &mut timeoutReceiver => {
                    timeout.map_err(|_| "tool permission timeout task was cancelled".to_string())?;
                    let mut state = self
                        .state
                        .lock()
                        .map_err(|error| format!("tool permission mutex poisoned: {error}"))?;
                    if let Some(result) = state.decisions.remove(&requestId) {
                        state.pending.remove(&requestId);
                        self.refreshRequests(&state);
                        return Ok(result);
                    }
                    state.pending.remove(&requestId);
                    self.refreshRequests(&state);
                    return Err(format!("chat tool permission timed out: {requestId}"));
                }
            }
        }
    }

    /// Applies a decision only when the request belongs to the addressed chat.
    fn respond(&self, chatId: &str, requestId: &str, result: String) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("tool permission mutex poisoned: {error}"))?;
        let Some(request) = state.pending.get(requestId) else {
            return Err(format!(
                "chat tool permission request not found: {requestId}"
            ));
        };
        if request.chatId != chatId {
            return Err(format!(
                "chat does not own tool permission request: {requestId}"
            ));
        }
        state.decisions.insert(requestId.to_string(), result);
        self.refreshRequests(&state);
        self.changed.notify_waiters();
        Ok(())
    }
}

/// Service facade for publishing and responding to host interaction requests.
pub struct RuntimeHostInteractionService {
    systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
    httpHost: Option<Arc<dyn operit_host_api::HttpHost>>,
}

/// A terminal image download result transported through the runtime proxy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeImageData {
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
    pub displayUrl: Option<String>,
    pub error: Option<String>,
}

/// A cold image source whose collection lifetime owns any host byte request.
pub struct RuntimeImageStream {
    url: String,
    host: Option<Arc<dyn operit_host_api::HttpHost>>,
}

/// Closes the host request when collection completes or is cancelled.
struct RuntimeImageRequest {
    id: String,
    host: Arc<dyn operit_host_api::HttpHost>,
}

impl Drop for RuntimeImageRequest {
    /// Releases the request, including when the proxy drops its collector.
    fn drop(&mut self) {
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}

/// Accumulates at most sixteen MiB without buffering an unbounded chunk queue.
#[derive(Default)]
struct RuntimeImageBuffer {
    bytes: Vec<u8>,
    completion: Option<oneshot::Sender<Result<Vec<u8>, String>>>,
}

impl RuntimeImageBuffer {
    /// Accepts a chunk or terminates the request when its byte budget is exceeded.
    fn push(&mut self, chunk: Vec<u8>) {
        if self.completion.is_none() {
            return;
        }
        if chunk.len() > 16 * 1024 * 1024 - self.bytes.len() {
            self.finish(Err("Image exceeded 16 MiB size limit".to_string()));
            return;
        }
        self.bytes.extend_from_slice(&chunk);
    }

    /// Publishes exactly one terminal result and releases the buffered bytes.
    fn finish(&mut self, result: Result<(), String>) {
        if let Some(completion) = self.completion.take() {
            let bytes = std::mem::take(&mut self.bytes);
            let result = result.and_then(|()| {
                if bytes.is_empty() {
                    Err("Image response was empty".to_string())
                } else {
                    Ok(bytes)
                }
            });
            let _ = completion.send(result);
        }
    }
}

#[cfg(test)]
mod runtimeImageBufferTests {
    use super::RuntimeImageBuffer;
    use tokio::sync::oneshot;

    /// Verifies the byte limit rejects excess data before retaining it.
    #[test]
    fn rejectsOversizedImage() {
        let (sender, mut receiver) = oneshot::channel();
        let mut buffer = RuntimeImageBuffer {
            bytes: vec![0; 16 * 1024 * 1024],
            completion: Some(sender),
        };
        buffer.push(vec![1]);
        assert_eq!(
            receiver.try_recv().unwrap().unwrap_err(),
            "Image exceeded 16 MiB size limit"
        );
        assert!(buffer.bytes.is_empty());
        buffer.push(vec![2]);
        assert!(buffer.bytes.is_empty());
    }

    /// Verifies ordered chunks are delivered only after successful completion.
    #[test]
    fn completesOrderedImage() {
        let (sender, mut receiver) = oneshot::channel();
        let mut buffer = RuntimeImageBuffer {
            bytes: Vec::new(),
            completion: Some(sender),
        };
        buffer.push(vec![1, 2]);
        buffer.push(vec![3]);
        buffer.finish(Ok(()));
        assert_eq!(receiver.try_recv().unwrap().unwrap(), vec![1, 2, 3]);
    }

    /// Verifies an empty successful response cannot become an image result.
    #[test]
    fn rejectsEmptyImage() {
        let (sender, mut receiver) = oneshot::channel();
        let mut buffer = RuntimeImageBuffer {
            bytes: Vec::new(),
            completion: Some(sender),
        };
        buffer.finish(Ok(()));
        assert_eq!(
            receiver.try_recv().unwrap().unwrap_err(),
            "Image response was empty"
        );
    }
}

impl RuntimeImageStream {
    /// Resolves image delivery through the host and bounds byte-stream downloads.
    async fn load(&self) -> Result<RuntimeImageData, String> {
        let url = url::Url::parse(&self.url).map_err(|error| error.to_string())?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err("Image URL must use HTTP or HTTPS".to_string());
        }
        let host = self.host.clone().ok_or("HTTP host is not registered")?;
        match host.imageDelivery() {
            operit_host_api::HttpImageDelivery::DisplayUrl => {
                return Ok(RuntimeImageData {
                    bytes: Vec::new(),
                    displayUrl: Some(url.to_string()),
                    error: None,
                });
            }
            operit_host_api::HttpImageDelivery::Bytes => {}
        }
        let request = RuntimeImageRequest {
            id: format!("markdown-image-{}", Uuid::new_v4()),
            host,
        };
        let (sender, receiver) = oneshot::channel();
        let buffer = Arc::new(Mutex::new(RuntimeImageBuffer {
            bytes: Vec::new(),
            completion: Some(sender),
        }));
        let chunkBuffer = buffer.clone();
        request
            .host
            .openHttpByteStream(
                request.id.clone(),
                operit_host_api::HttpRequestData {
                    url: url.to_string(),
                    method: "GET".to_string(),
                    headers: Vec::new(),
                    body: Vec::new(),
                    formFields: Vec::new(),
                    fileParts: Vec::new(),
                    connectTimeoutSeconds: 20,
                    readTimeoutSeconds: 30,
                    followRedirects: true,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                },
                Arc::new(|| {}),
                Arc::new(move |chunk| chunkBuffer.lock().expect("image buffer lock").push(chunk)),
                Arc::new(move |result| buffer.lock().expect("image buffer lock").finish(result)),
            )
            .map_err(|error| error.message)?;
        let scheduler = defaultHostRuntimeTaskSchedulerHost();
        let timeout = scheduler.waitForHostRuntimeDelay(30_000);
        let bytes = tokio::select! {
            result = receiver => result.map_err(|error| error.to_string())?,
            _ = timeout => Err("Image download timed out after 30 seconds".to_string()),
        }?;
        Ok(RuntimeImageData {
            bytes,
            displayUrl: None,
            error: None,
        })
    }
}

impl Stream for RuntimeImageStream {
    type Item = RuntimeImageData;

    /// Emits one result and cancels the HTTP request when collection is dropped.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        Box::pin(async move {
            let result = match self.load().await {
                Ok(data) => data,
                Err(error) => RuntimeImageData {
                    bytes: Vec::new(),
                    displayUrl: None,
                    error: Some(error),
                },
            };
            collector(result);
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Bytes and metadata returned for a registered image or media-pool entry.
pub struct RuntimeMediaPoolData {
    pub mediaType: String,
    pub base64: String,
    pub mimeType: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug)]
/// Blocking stream of pending host interaction requests for selected kinds.
pub struct RuntimeHostInteractionEventStream {
    kinds: Vec<RuntimeHostInteractionKind>,
}

impl Stream for RuntimeHostInteractionEventStream {
    type Item = RuntimeHostInteractionRequest;

    /// Collects matching pending interactions in broker insertion order.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        Box::pin(async move {
            let broker = runtimeHostInteractionBroker();
            let mut delivered = BTreeSet::<String>::new();
            loop {
                let changed = broker.asyncChanged.notified();
                let next = broker
                    .state
                    .lock()
                    .expect("host interaction mutex poisoned")
                    .pending
                    .values()
                    .filter(|pending| {
                        !delivered.contains(&pending.request.requestId)
                            && self.matchesPending(pending)
                    })
                    .min_by_key(|pending| pending.sequence)
                    .map(|pending| pending.request.clone());
                match next {
                    Some(request) => {
                        delivered.insert(request.requestId.clone());
                        collector(request);
                    }
                    None => changed.await,
                }
            }
        })
    }
}

impl RuntimeHostInteractionService {
    /// Creates the host interaction service facade.
    pub fn getInstance(context: &HostManager) -> Self {
        Self {
            systemOperationHost: context.systemOperationHost.clone(),
            httpHost: context.httpHost.clone(),
        }
    }

    /// Creates a cancellable image download through the active runtime HTTP host.
    pub fn imageBytes(&self, url: String) -> RuntimeImageStream {
        RuntimeImageStream {
            url,
            host: self.httpHost.clone(),
        }
    }

    /// Reads one registered image or audio/video entry by its pool id.
    pub fn getMediaPoolData(
        &self,
        mediaType: String,
        id: String,
    ) -> Result<RuntimeMediaPoolData, String> {
        let mediaType = mediaType.trim().to_ascii_lowercase();
        let id = id.trim();
        if id.is_empty() {
            return Err("media pool id is required".to_string());
        }
        match mediaType.as_str() {
            "image" => {
                let image = ImagePoolManager::get_image(id)
                    .ok_or_else(|| format!("image pool entry not found: {id}"))?;
                if !image.mime_type.to_ascii_lowercase().starts_with("image/") {
                    return Err(format!(
                        "image pool entry has invalid MIME type: {}",
                        image.mime_type
                    ));
                }
                Ok(RuntimeMediaPoolData {
                    mediaType,
                    base64: image.base64,
                    mimeType: image.mime_type,
                    width: image.width,
                    height: image.height,
                })
            }
            "audio" | "video" | "file" => {
                let media = MediaPoolManager::get_media(id)
                    .ok_or_else(|| format!("media pool entry not found: {id}"))?;
                let mimeType = media.mime_type.to_ascii_lowercase();
                let expectedPrefix = format!("{mediaType}/");
                if mediaType != "file" && !mimeType.starts_with(&expectedPrefix) {
                    return Err(format!(
                        "{mediaType} pool entry has incompatible MIME type: {}",
                        media.mime_type
                    ));
                }
                Ok(RuntimeMediaPoolData {
                    mediaType,
                    base64: media.base64,
                    mimeType: media.mime_type,
                    width: 0,
                    height: 0,
                })
            }
            _ => Err(format!("unsupported media pool type: {mediaType}")),
        }
    }

    /// Sends one application notification through the active system-operation host.
    pub fn sendSystemNotification(
        &self,
        title: String,
        message: String,
        chatId: Option<String>,
    ) -> Result<(), String> {
        let host = self
            .systemOperationHost
            .as_deref()
            .ok_or_else(|| "SystemOperationHost is not registered for this runtime".to_string())?;
        let activation = match chatId {
            Some(chatId) => SystemNotificationActivation::OpenChat { chatId },
            None => SystemNotificationActivation::OpenApplication,
        };
        host.sendNotification(&SystemNotificationRequest {
            title,
            message,
            activation,
        })
        .map_err(|error| error.message)
    }

    /// Responds to a pending owner-host interaction request.
    pub fn respondOwnerHostInteraction(
        &self,
        requestId: String,
        response: RuntimeHostInteractionResponse,
    ) -> Result<(), String> {
        runtimeHostInteractionBroker().respond(&requestId, response)
    }

    /// Acknowledges a non-blocking owner-host notification after it is consumed.
    pub fn acknowledgeOwnerHostInteraction(&self, requestId: String) -> Result<(), String> {
        runtimeHostInteractionBroker().acknowledge(&requestId)
    }

    /// Creates an event stream for owner-host interaction requests of selected kinds.
    pub fn ownerHostInteractionEvents(
        &self,
        kinds: Vec<RuntimeHostInteractionKind>,
    ) -> RuntimeHostInteractionEventStream {
        RuntimeHostInteractionEventStream { kinds }
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

/// Requests a browser session command from the owner host and waits for a response.
pub fn requestOwnerBrowserSession(
    payload: RuntimeHostInteractionBrowserSessionPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionBrowserSessionResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::browserSession(payload),
        timeout,
    )?;
    response
        .browserSession
        .ok_or_else(|| "browser session response payload is missing".to_string())
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

/// Requests one Compose DSL file-picker interaction from the owner host.
pub fn requestOwnerComposeFilePicker(
    payload: RuntimeHostInteractionComposeFilePickerPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionComposeFilePickerResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::composeFilePicker(payload),
        timeout,
    )?;
    response
        .composeFilePicker
        .ok_or_else(|| "compose file picker response payload is missing".to_string())
}

/// Requests an export through the owner host file-save selector.
pub fn requestOwnerComposeFileSave(
    payload: RuntimeHostInteractionComposeFileSavePayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionComposeFileSaveResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::composeFileSave(payload),
        timeout,
    )?;
    response.composeFileSave.ok_or_else(|| "compose file save response payload is missing".to_string())
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

/// Requests the current system language code from the owner host.
pub fn requestOwnerSystemLanguageCode(
    timeout: Duration,
) -> Result<RuntimeHostInteractionSystemLanguageCodeResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::systemLanguageCode(),
        timeout,
    )?;
    response
        .systemLanguageCode
        .ok_or_else(|| "system language code response payload is missing".to_string())
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

/// Requests a system operation from the owner host.
pub fn requestOwnerSystemOperation(
    payload: RuntimeHostInteractionSystemOperationPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionSystemOperationResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::systemOperation(payload),
        timeout,
    )?;
    response
        .systemOperation
        .ok_or_else(|| "system operation response payload is missing".to_string())
}

/// Requests file opening from the owner host.
pub fn requestOwnerFileOpen(
    payload: RuntimeHostInteractionFileOpenPayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionFileOperationResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::fileOpen(payload),
        timeout,
    )?;
    response
        .fileOpen
        .ok_or_else(|| "file open response payload is missing".to_string())
}

/// Requests file sharing from the owner host.
pub fn requestOwnerFileShare(
    payload: RuntimeHostInteractionFileSharePayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionFileOperationResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::fileShare(payload),
        timeout,
    )?;
    response
        .fileShare
        .ok_or_else(|| "file share response payload is missing".to_string())
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

/// Requests local inference execution from the owner host.
pub fn requestOwnerLocalInference(
    payload: RuntimeHostInteractionLocalInferencePayload,
    timeout: Duration,
) -> Result<RuntimeHostInteractionLocalInferenceResponse, String> {
    let response = runtimeHostInteractionBroker().request(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::localInference(payload),
        timeout,
    )?;
    response
        .localInference
        .ok_or_else(|| "local inference response payload is missing".to_string())
}

/// Returns the permission requests currently pending for one chat.
pub fn chatToolPermissionRequestsFlow(
    chatId: String,
) -> StateFlow<Vec<RuntimeHostInteractionToolPermissionRequest>> {
    chatToolPermissionBroker()
        .requests
        .asStateFlow()
        .map(move |requests| {
            requests
                .into_iter()
                .filter(|request| request.chatId == chatId)
                .collect()
        })
}

/// Responds to a permission request through its owning chat scope.
pub fn respondChatToolPermission(
    chatId: String,
    requestId: String,
    result: String,
) -> Result<(), String> {
    chatToolPermissionBroker().respond(&chatId, &requestId, result)
}

/// Publishes a permission request into its chat state and awaits the chat decision.
pub async fn requestChatToolPermissionAsync(
    chatId: String,
    tool: RuntimeHostInteractionToolPermissionTool,
    description: String,
    timeout: Duration,
) -> Result<String, String> {
    chatToolPermissionBroker()
        .request(chatId, tool, description, timeout)
        .await
}



/// Publishes one application notification event to the owner host.
pub fn publishOwnerAppNotification(payload: RuntimeHostInteractionAppNotificationPayload) {
    runtimeHostInteractionBroker().publish(
        RuntimeHostInteractionTarget::OwnerHost,
        RuntimeHostInteractionRequest::appNotification(payload),
    );
}

fn runtimeHostInteractionBroker() -> &'static RuntimeHostInteractionBroker {
    RUNTIME_HOST_INTERACTIONS.get_or_init(RuntimeHostInteractionBroker::default)
}

impl RuntimeHostInteractionEventStream {
    fn matchesPending(&self, pending: &RuntimeHostInteractionPending) -> bool {
        if !self.kinds.iter().any(|kind| kind == &pending.request.kind) {
            return false;
        }
        pending.target == RuntimeHostInteractionTarget::OwnerHost
    }
}

impl RuntimeHostInteractionBroker {
    /// Wakes synchronous and asynchronous observers after broker state changes.
    fn notifyChanged(&self) {
        self.changed.notify_all();
        self.asyncChanged.notify_waiters();
    }

    /// Enqueues one interaction and waits for its response or timeout.
    fn request(
        &self,
        target: RuntimeHostInteractionTarget,
        request: RuntimeHostInteractionRequest,
        timeout: Duration,
    ) -> Result<RuntimeHostInteractionResponse, String> {
        let startedAt = tryCurrentTimeMillisU128()?;
        let requestId = request.requestId.clone();
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
        let sequence = state.nextSequence;
        state.nextSequence = state.nextSequence.wrapping_add(1);
        state.pending.insert(
            requestId.clone(),
            RuntimeHostInteractionPending {
                target,
                request,
                sequence,
                awaitsResponse: true,
            },
        );
        self.notifyChanged();
        loop {
            if let Some(response) = state.responses.remove(&requestId) {
                state.pending.remove(&requestId);
                self.notifyChanged();
                if let Some(error) = response.error.as_ref() {
                    return Err(error.clone());
                }
                return Ok(response);
            }
            let elapsedMillis = tryCurrentTimeMillisU128()?.saturating_sub(startedAt);
            let elapsed = Duration::from_millis(
                u64::try_from(elapsedMillis)
                    .map_err(|_| "host interaction elapsed time exceeds u64 milliseconds")?,
            );
            if elapsed >= timeout {
                state.pending.remove(&requestId);
                self.notifyChanged();
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

    /// Enqueues one interaction and asynchronously waits for its response or timeout.
    async fn requestAsync(
        &self,
        target: RuntimeHostInteractionTarget,
        request: RuntimeHostInteractionRequest,
        timeout: Duration,
    ) -> Result<RuntimeHostInteractionResponse, String> {
        let requestId = request.requestId.clone();
        {
            let mut state = self
                .state
                .lock()
                .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
            let sequence = state.nextSequence;
            state.nextSequence = state.nextSequence.wrapping_add(1);
            state.pending.insert(
                requestId.clone(),
                RuntimeHostInteractionPending {
                    target,
                    request,
                    sequence,
                    awaitsResponse: true,
                },
            );
        }
        self.notifyChanged();

        let timeoutMillis = u64::try_from(timeout.as_millis())
            .map_err(|_| "host interaction timeout exceeds u64 milliseconds")?;
        let (timeoutSender, mut timeoutReceiver) = oneshot::channel();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleDelayedHostRuntimeTask(
                "runtime-host-interaction-timeout",
                timeoutMillis,
                Box::new(move || {
                    let _ = timeoutSender.send(());
                }),
            )
            .map_err(|error| error.message)?;

        loop {
            let changed = self.asyncChanged.notified();
            {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
                if let Some(response) = state.responses.remove(&requestId) {
                    state.pending.remove(&requestId);
                    self.notifyChanged();
                    if let Some(error) = response.error.as_ref() {
                        return Err(error.clone());
                    }
                    return Ok(response);
                }
            }

            tokio::select! {
                _ = changed => {}
                timeout = &mut timeoutReceiver => {
                    timeout.map_err(|_| "host interaction timeout task was cancelled".to_string())?;
                    let mut state = self
                        .state
                        .lock()
                        .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
                    if let Some(response) = state.responses.remove(&requestId) {
                        state.pending.remove(&requestId);
                                        self.notifyChanged();
                        if let Some(error) = response.error.as_ref() {
                            return Err(error.clone());
                        }
                        return Ok(response);
                    }
                    state.pending.remove(&requestId);
                                self.notifyChanged();
                    return Err(format!("host interaction timed out: {requestId}"));
                }
            }
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
        let Some(pending) = state.pending.get(requestId) else {
            return Err(format!("host interaction request not found: {requestId}"));
        };
        if !pending.awaitsResponse {
            return Err(format!(
                "host interaction notification must be acknowledged: {requestId}"
            ));
        }
        state.responses.insert(requestId.to_string(), response);
        self.notifyChanged();
        Ok(())
    }

    /// Enqueues one non-blocking notification until the owner host acknowledges it.
    fn publish(
        &self,
        target: RuntimeHostInteractionTarget,
        request: RuntimeHostInteractionRequest,
    ) {
        let requestId = request.requestId.clone();
        let mut state = self.state.lock().expect("host interaction mutex poisoned");
        let sequence = state.nextSequence;
        state.nextSequence = state.nextSequence.wrapping_add(1);
        state.pending.insert(
            requestId,
            RuntimeHostInteractionPending {
                target,
                request,
                sequence,
                awaitsResponse: false,
            },
        );
        self.notifyChanged();
    }

    /// Removes one non-blocking notification after the owner host consumes it.
    fn acknowledge(&self, requestId: &str) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("host interaction mutex poisoned: {error}"))?;
        let Some(pending) = state.pending.get(requestId) else {
            return Err(format!("host interaction request not found: {requestId}"));
        };
        if pending.awaitsResponse {
            return Err(format!("host interaction requires a response: {requestId}"));
        }
        state.pending.remove(requestId);
        self.notifyChanged();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies image and audio/video pool entries are returned by pool id.
    #[test]
    fn mediaPoolDataIsReadByIdWithTypeValidation() {
        let service = RuntimeHostInteractionService::getInstance(&HostManager::new());
        let imageId = ImagePoolManager::add_image_bytes(
            b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01",
            Some("image/png"),
            None,
        );
        let audioId = MediaPoolManager::add_media_bytes(b"audio", "audio/mpeg");
        let videoId = MediaPoolManager::add_media_bytes(b"video", "video/mp4");
        let invalidAudioId = MediaPoolManager::add_media_bytes(b"audio", "text/plain");

        let image = service
            .getMediaPoolData("image".to_string(), imageId.clone())
            .expect("image pool entry must be readable");
        let audio = service
            .getMediaPoolData("audio".to_string(), audioId.clone())
            .expect("audio pool entry must be readable");
        let video = service
            .getMediaPoolData("video".to_string(), videoId.clone())
            .expect("video pool entry must be readable");

        assert_eq!(image.mimeType, "image/png");
        assert!(!image.base64.is_empty());
        assert_eq!(audio.mimeType, "audio/mpeg");
        assert_eq!(audio.base64, "YXVkaW8=");
        assert_eq!(video.mimeType, "video/mp4");
        assert_eq!(video.base64, "dmlkZW8=");
        assert!(service
            .getMediaPoolData("audio".to_string(), invalidAudioId.clone())
            .is_err());

        MediaPoolManager::remove_media(&audioId);
        MediaPoolManager::remove_media(&videoId);
        MediaPoolManager::remove_media(&invalidAudioId);
    }



    /// Verifies that a structured owner error returns without waiting for timeout.
    #[test]
    fn brokerReturnsStructuredOwnerErrorImmediately() {
        let broker = RuntimeHostInteractionBroker::default();
        let mut request =
            RuntimeHostInteractionRequest::ttsPlayback(RuntimeHostInteractionTtsPlaybackPayload {
                command: "speak".to_string(),
                audioPath: None,
                text: "hello".to_string(),
                voice: String::new(),
                locale: String::new(),
                speed: 1.0,
                pitch: 1.0,
                interrupt: true,
            });
        request.requestId = "structured-owner-error".to_string();
        let mut response = RuntimeHostInteractionResponse::empty();
        response.error = Some("owner tts failed".to_string());
        broker
            .state
            .lock()
            .expect("host interaction mutex poisoned")
            .responses
            .insert(request.requestId.clone(), response);

        let error = broker
            .request(
                RuntimeHostInteractionTarget::OwnerHost,
                request,
                Duration::from_secs(120),
            )
            .expect_err("structured owner error must fail the request");

        assert_eq!(error, "owner tts failed");
    }
}
