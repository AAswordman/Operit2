//! Typed owner adapters implementing the existing host capability interfaces.
//! No platform selection, string heuristics, or alternate backend policy belongs here.

use std::sync::Arc;
use std::time::Duration;

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, AudioPlaybackHost, AudioPlaybackStatus,
    DeviceInfoData, HostError, HostResult, LocalInferenceHost, LocalSttInferenceHostRequest,
    LocalSttInferenceHostResponse, LocalTtsInferenceHostRequest, LocalTtsInferenceHostResponse,
    LocationData, MusicPlaybackRequest, MusicPlaybackStatus, NotificationData, OCRLanguage,
    OCRQuality, SystemNotificationRequest, SystemOperationHost, SystemSettingData, TtsPlaybackHost,
    TtsPlaybackRequest, TtsPlaybackStatus, TtsSynthesisHost, TtsSynthesisRequest,
    TtsSynthesisResponse,
};
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerAudioPlay, requestOwnerBluetooth, requestOwnerFileOpen, requestOwnerFileShare,
    requestOwnerLocalInference, requestOwnerMusicPlayback, requestOwnerSystemCaptureScreenshot,
    requestOwnerSystemOperation, requestOwnerSystemRecognizeText, requestOwnerTtsPlayback,
    requestOwnerTtsSynthesis, RuntimeHostInteractionAudioPlayPayload,
    RuntimeHostInteractionBluetoothPayload, RuntimeHostInteractionFileOpenPayload,
    RuntimeHostInteractionFileSharePayload, RuntimeHostInteractionLocalInferencePayload,
    RuntimeHostInteractionMusicPlaybackPayload, RuntimeHostInteractionSystemOperationPayload,
    RuntimeHostInteractionSystemRecognizeTextPayload, RuntimeHostInteractionTtsPlaybackPayload,
    RuntimeHostInteractionTtsSynthesisPayload,
};

/// Executes one owner system operation with its exact expected response schema.
pub(crate) fn ownerSystemOperation<T: serde::de::DeserializeOwned>(
    operation: &str,
    params: serde_json::Value,
) -> HostResult<T> {
    let response = requestOwnerSystemOperation(
        RuntimeHostInteractionSystemOperationPayload {
            operation: operation.to_string(),
            paramsJson: serde_json::to_string(&params).map_err(|error| {
                HostError::new(format!("{operation} request encode failed: {error}"))
            })?,
        },
        Duration::from_secs(120),
    )
    .map_err(HostError::new)?;
    serde_json::from_str(&response.resultJson)
        .map_err(|error| HostError::new(format!("{operation} response decode failed: {error}")))
}

/// Requires an explicit success acknowledgement from an owner operation.
pub(crate) fn ownerConfirmed(operation: &str, params: serde_json::Value) -> HostResult<()> {
    #[derive(serde::Deserialize)]
    struct Confirmation {
        success: bool,
    }
    let response: Confirmation = ownerSystemOperation(operation, params)?;
    if !response.success {
        return Err(HostError::new(format!(
            "{operation} was not confirmed by the owner"
        )));
    }
    Ok(())
}

/// Presents an application toast through the existing owner interaction boundary.
pub(crate) fn presentFlutterToast(message: &str) -> HostResult<()> {
    ownerConfirmed("toast", serde_json::json!({ "message": message }))
}

/// Writes system clipboard text through the owner interaction boundary.
pub(crate) fn setFlutterClipboardText(text: &str) -> HostResult<String> {
    #[derive(serde::Deserialize)]
    struct ClipboardWrite {
        success: bool,
        text: String,
    }
    let response: ClipboardWrite = ownerSystemOperation(
        "set_clipboard_text",
        serde_json::json!({ "text": text }),
    )?;
    if !response.success {
        return Err(HostError::new(
            "set_clipboard_text was not confirmed by the owner",
        ));
    }
    Ok(response.text)
}

/// Reads a location from the owner that can display its authorization UI.
pub(crate) fn ownerLocation(
    timeout: i32,
    highAccuracy: bool,
    includeAddress: bool,
) -> HostResult<LocationData> {
    ownerSystemOperation(
        "get_device_location",
        serde_json::json!({
            "timeout": timeout, "highAccuracy": highAccuracy, "includeAddress": includeAddress,
        }),
    )
}

/// Reads a location through the foreground owner permission boundary.
pub(crate) fn ownerLocationForeground(
    timeout: i32,
    highAccuracy: bool,
    includeAddress: bool,
) -> HostResult<LocationData> {
    ownerSystemOperation(
        "get_device_location_foreground",
        serde_json::json!({
            "timeout": timeout, "highAccuracy": highAccuracy, "includeAddress": includeAddress,
        }),
    )
}

/// Sends a notification through the owner's authorization UI.
pub(crate) fn ownerSendNotification(request: &SystemNotificationRequest) -> HostResult<()> {
    ownerConfirmed(
        "send_notification",
        serde_json::json!({
            "title": request.title, "message": request.message, "activation": request.activation,
        }),
    )
}

/// Reads notifications through an explicitly authorized owner listener.
pub(crate) fn ownerNotifications(limit: i32, includeOngoing: bool) -> HostResult<NotificationData> {
    ownerSystemOperation(
        "get_notifications",
        serde_json::json!({ "limit": limit, "includeOngoing": includeOngoing }),
    )
}

/// Reads a system setting through the owner's exact settings response schema.
pub(crate) fn ownerGetSystemSetting(namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
    ownerSystemSetting(
        "get_system_setting",
        serde_json::json!({ "namespace": namespace, "setting": setting }),
    )
}

/// Modifies a system setting through the owner's settings authorization boundary.
pub(crate) fn ownerModifySystemSetting(
    namespace: &str,
    setting: &str,
    value: &str,
) -> HostResult<SystemSettingData> {
    ownerSystemSetting(
        "modify_system_setting",
        serde_json::json!({ "namespace": namespace, "setting": setting, "value": value }),
    )
}

/// Decodes the owner's required setting fields into the shared host data type.
fn ownerSystemSetting(operation: &str, params: serde_json::Value) -> HostResult<SystemSettingData> {
    #[derive(serde::Deserialize)]
    struct SettingResponse {
        namespace: String,
        setting: String,
        value: String,
    }
    let response: SettingResponse = ownerSystemOperation(operation, params)?;
    Ok(SystemSettingData {
        namespace: response.namespace,
        setting: response.setting,
        value: response.value,
    })
}

/// Reads device information through the owner system API.
pub(crate) fn ownerDeviceInfo() -> HostResult<DeviceInfoData> {
    ownerSystemOperation("get_device_info", serde_json::json!({}))
}

/// Captures a screenshot through the owner screen capture boundary.
pub(crate) fn ownerScreenshot() -> HostResult<String> {
    requestOwnerSystemCaptureScreenshot(Duration::from_secs(60))
        .map(|response| response.path)
        .map_err(HostError::new)
}

/// Recognizes text through the owner OCR boundary.
pub(crate) fn ownerRecognizeText(
    imagePath: &str,
    language: OCRLanguage,
    quality: OCRQuality,
) -> HostResult<String> {
    requestOwnerSystemRecognizeText(
        RuntimeHostInteractionSystemRecognizeTextPayload {
            imagePath: imagePath.to_string(),
            language: language.asHostValue().to_string(),
            quality: quality.asHostValue().to_string(),
        },
        Duration::from_secs(60),
    )
    .map(|response| response.text)
    .map_err(HostError::new)
}

/// Forwards a typed Bluetooth command without interpreting or rewriting its operation name.
pub(crate) fn ownerBluetooth(
    command: &str,
    params: serde_json::Value,
) -> HostResult<serde_json::Value> {
    let response = requestOwnerBluetooth(
        RuntimeHostInteractionBluetoothPayload {
            command: command.to_string(),
            paramsJson: serde_json::to_string(&params).map_err(|error| {
                HostError::new(format!("Bluetooth request encode failed: {error}"))
            })?,
        },
        Duration::from_secs(120),
    )
    .map_err(HostError::new)?;
    serde_json::from_str(&response.resultJson)
        .map_err(|error| HostError::new(format!("Bluetooth response decode failed: {error}")))
}

/// Opens an owner-visible file using the installed platform file capability.
pub(crate) fn ownerFileOpen(path: &str) -> HostResult<()> {
    let response = requestOwnerFileOpen(
        RuntimeHostInteractionFileOpenPayload {
            path: path.to_string(),
        },
        Duration::from_secs(60),
    )
    .map_err(HostError::new)?;
    if response.success {
        return Ok(());
    }
    Err(HostError::new(response.error.ok_or_else(|| {
        HostError::new("File open response omitted its error")
    })?))
}

/// Shares an owner-visible file using the installed platform sharing capability.
pub(crate) fn ownerFileShare(path: &str, title: &str) -> HostResult<()> {
    let response = requestOwnerFileShare(
        RuntimeHostInteractionFileSharePayload {
            path: path.to_string(),
            title: title.to_string(),
        },
        Duration::from_secs(60),
    )
    .map_err(HostError::new)?;
    if response.success {
        return Ok(());
    }
    Err(HostError::new(response.error.ok_or_else(|| {
        HostError::new("File share response omitted its error")
    })?))
}

/// Selects concrete owner callbacks once, during host assembly.
pub(crate) struct FlutterSystemBindings {
    pub notificationSender: Arc<dyn Fn(&SystemNotificationRequest) -> HostResult<()> + Send + Sync>,
    pub notifications: Arc<dyn Fn(i32, bool) -> HostResult<NotificationData> + Send + Sync>,
    pub location: Arc<dyn Fn(i32, bool, bool) -> HostResult<LocationData> + Send + Sync>,
    pub deviceInfo: Arc<dyn Fn() -> HostResult<DeviceInfoData> + Send + Sync>,
    pub screenshot: Arc<dyn Fn() -> HostResult<String> + Send + Sync>,
    pub recognition: Arc<dyn Fn(&str, OCRLanguage, OCRQuality) -> HostResult<String> + Send + Sync>,
}

/// Adds owner authorization and UI operations to an already selected system host.
pub(crate) struct FlutterSystemOperationBridge {
    host: Arc<dyn SystemOperationHost>,
    bindings: FlutterSystemBindings,
}

impl FlutterSystemOperationBridge {
    /// Creates a fixed binding set without choosing another backend after failures.
    pub(crate) fn new(host: Arc<dyn SystemOperationHost>, bindings: FlutterSystemBindings) -> Self {
        Self { host, bindings }
    }
}

impl SystemOperationHost for FlutterSystemOperationBridge {
    /// Reads the selected host's language.
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        self.host.getSystemLanguageCode()
    }
    /// Sends a system notification using the owner's authorization UI.
    fn sendNotification(&self, request: &SystemNotificationRequest) -> HostResult<()> {
        (self.bindings.notificationSender)(request)
    }
    /// Modifies settings through the selected system host.
    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData> {
        self.host.modifySystemSetting(namespace, setting, value)
    }
    /// Reads settings through the selected system host.
    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData> {
        self.host.getSystemSetting(namespace, setting)
    }
    /// Installs an app through the selected system host.
    fn installApp(&self, path: &str) -> HostResult<AppOperationData> {
        self.host.installApp(path)
    }
    /// Uninstalls an app through the selected system host.
    fn uninstallApp(&self, name: &str) -> HostResult<AppOperationData> {
        self.host.uninstallApp(name)
    }
    /// Lists apps through the selected system host.
    fn listInstalledApps(&self, includeSystem: bool) -> HostResult<AppListData> {
        self.host.listInstalledApps(includeSystem)
    }
    /// Starts an app through the selected system host.
    fn startApp(&self, name: &str) -> HostResult<AppOperationData> {
        self.host.startApp(name)
    }
    /// Stops an app through the selected system host.
    fn stopApp(&self, name: &str) -> HostResult<AppOperationData> {
        self.host.stopApp(name)
    }
    /// Reads notifications through the callback selected at host assembly.
    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData> {
        (self.bindings.notifications)(limit, includeOngoing)
    }
    /// Reads app usage through the selected system host.
    fn getAppUsageTime(
        &self,
        name: &str,
        hours: i32,
        limit: i32,
        includeSystem: bool,
    ) -> HostResult<AppUsageTimeResultData> {
        self.host.getAppUsageTime(name, hours, limit, includeSystem)
    }
    /// Reads current location through the owner's authorization UI.
    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData> {
        (self.bindings.location)(timeout, highAccuracy, includeAddress)
    }
    /// Reads information through the callback selected at host assembly.
    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData> {
        (self.bindings.deviceInfo)()
    }
    /// Captures a screen through the callback selected at host assembly.
    fn captureScreenshot(&self) -> HostResult<String> {
        (self.bindings.screenshot)()
    }
    /// Recognizes text through the owner's OCR implementation.
    fn recognizeText(
        &self,
        path: &str,
        language: OCRLanguage,
        quality: OCRQuality,
    ) -> HostResult<String> {
        (self.bindings.recognition)(path, language, quality)
    }
}

/// Implements audio and music directly against typed host methods.
pub(crate) struct FlutterAudioPlaybackHost;

impl FlutterAudioPlaybackHost {
    /// Sends one typed music payload and decodes the exact host status fields.
    fn music(
        payload: RuntimeHostInteractionMusicPlaybackPayload,
    ) -> HostResult<MusicPlaybackStatus> {
        let result =
            requestOwnerMusicPlayback(payload, Duration::from_secs(60)).map_err(HostError::new)?;
        Ok(MusicPlaybackStatus {
            state: result.state,
            source: result.source,
            sourceType: result.sourceType,
            title: result.title,
            artist: result.artist,
            durationMs: result.durationMs,
            positionMs: result.positionMs,
            bufferedPositionMs: result.bufferedPositionMs,
            volume: result.volume,
            loopPlayback: result.loopPlayback,
            message: result.message,
        })
    }
    /// Creates a control payload with no playback source or implicit action.
    fn control(command: &str) -> RuntimeHostInteractionMusicPlaybackPayload {
        RuntimeHostInteractionMusicPlaybackPayload {
            command: command.to_string(),
            source: None,
            sourceType: None,
            title: None,
            artist: None,
            loopPlayback: false,
            volume: 1.0,
            positionMs: 0,
        }
    }
}

impl AudioPlaybackHost for FlutterAudioPlaybackHost {
    /// Plays one explicit audio file through the owner.
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        let result = requestOwnerAudioPlay(
            RuntimeHostInteractionAudioPlayPayload {
                path: path.to_string(),
            },
            Duration::from_secs(60),
        )
        .map_err(HostError::new)?;
        Ok(AudioPlaybackStatus {
            path: result.path,
            started: result.started,
            details: result.details,
        })
    }
    /// Plays the exact requested music source.
    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus> {
        Self::music(RuntimeHostInteractionMusicPlaybackPayload {
            command: "play".to_string(),
            source: Some(request.source),
            sourceType: Some(request.sourceType),
            title: request.title,
            artist: request.artist,
            loopPlayback: request.loopPlayback,
            volume: request.volume,
            positionMs: request.startPositionMs,
        })
    }
    /// Pauses the current music session.
    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus> {
        Self::music(Self::control("pause"))
    }
    /// Resumes the current music session.
    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus> {
        Self::music(Self::control("resume"))
    }
    /// Stops the current music session.
    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus> {
        Self::music(Self::control("stop"))
    }
    /// Seeks the current session to the requested position.
    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus> {
        Self::music(RuntimeHostInteractionMusicPlaybackPayload {
            positionMs,
            ..Self::control("seek")
        })
    }
    /// Sets the current session's explicit volume.
    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus> {
        Self::music(RuntimeHostInteractionMusicPlaybackPayload {
            volume,
            ..Self::control("set_volume")
        })
    }
    /// Reads the current music status.
    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus> {
        Self::music(Self::control("status"))
    }
}

/// Implements speech playback without parsing a second command protocol inside Rust.
pub(crate) struct FlutterTtsPlaybackHost {
    pub systemSpeech: bool,
}

impl FlutterTtsPlaybackHost {
    /// Sends one speech operation with its exact typed payload.
    fn send(payload: RuntimeHostInteractionTtsPlaybackPayload) -> HostResult<TtsPlaybackStatus> {
        let result =
            requestOwnerTtsPlayback(payload, Duration::from_secs(120)).map_err(HostError::new)?;
        Ok(TtsPlaybackStatus {
            path: result.path,
            active: result.active,
            paused: result.paused,
            details: result.details,
        })
    }
    /// Creates a speech control operation without inventing audio or text input.
    fn control(command: &str) -> RuntimeHostInteractionTtsPlaybackPayload {
        RuntimeHostInteractionTtsPlaybackPayload {
            command: command.to_string(),
            audioPath: None,
            text: String::new(),
            voice: String::new(),
            locale: String::new(),
            speed: 1.0,
            pitch: 1.0,
            interrupt: false,
        }
    }
}

impl TtsPlaybackHost for FlutterTtsPlaybackHost {
    /// Reports the system speech capability selected during host assembly.
    fn supportsSystemSpeech(&self) -> bool {
        self.systemSpeech
    }
    /// Plays an explicit speech audio file.
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
        Self::send(RuntimeHostInteractionTtsPlaybackPayload {
            audioPath: Some(path.to_string()),
            interrupt: true,
            ..Self::control("play")
        })
    }
    /// Speaks an explicit request only when the selected owner supports system speech.
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        if !self.systemSpeech {
            return Err(HostError::new("This owner does not expose system speech"));
        }
        Self::send(RuntimeHostInteractionTtsPlaybackPayload {
            command: "speak".to_string(),
            audioPath: None,
            text: request.text,
            voice: request.voice,
            locale: request.locale,
            speed: request.speed,
            pitch: request.pitch,
            interrupt: request.interrupt,
        })
    }
    /// Pauses the current speech session.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        Self::send(Self::control("pause"))
    }
    /// Resumes the current speech session.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        Self::send(Self::control("resume"))
    }
    /// Stops the current speech session.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        Self::send(Self::control("stop"))
    }
    /// Reads the current speech session state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        Self::send(Self::control("state"))
    }
}

/// Synthesizes speech directly through the owner capability boundary.
pub(crate) struct FlutterTtsSynthesisHost;

impl TtsSynthesisHost for FlutterTtsSynthesisHost {
    /// Forwards the exact synthesis request to the authorized owner.
    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> {
        let result = requestOwnerTtsSynthesis(
            RuntimeHostInteractionTtsSynthesisPayload {
                text: request.text,
                voice: request.voice,
                locale: request.locale,
                speed: request.speed,
                pitch: request.pitch,
                outputFormat: request.outputFormat,
            },
            Duration::from_secs(120),
        )
        .map_err(HostError::new)?;
        Ok(TtsSynthesisResponse {
            audioPath: result.audioPath,
            details: result.details,
        })
    }
}

/// Executes local inference through the owner's installed engine, never another provider.
pub(crate) struct FlutterLocalInferenceHost;

impl FlutterLocalInferenceHost {
    /// Executes one typed local inference operation and validates its response schema.
    fn execute<T: serde::de::DeserializeOwned>(
        method: &str,
        request: impl serde::Serialize,
    ) -> HostResult<T> {
        let requestJson = serde_json::to_string(&request).map_err(|error| {
            HostError::new(format!("Local inference request encode failed: {error}"))
        })?;
        let result = requestOwnerLocalInference(
            RuntimeHostInteractionLocalInferencePayload {
                method: method.to_string(),
                requestJson,
            },
            Duration::from_secs(600),
        )
        .map_err(HostError::new)?;
        serde_json::from_str(&result.resultJson).map_err(|error| {
            HostError::new(format!("Local inference response decode failed: {error}"))
        })
    }
}

impl LocalInferenceHost for FlutterLocalInferenceHost {
    /// Transcribes the requested audio using the owner's local model engine.
    fn transcribeLocalSpeech(
        &self,
        request: LocalSttInferenceHostRequest,
    ) -> HostResult<LocalSttInferenceHostResponse> {
        Self::execute("transcribeLocalSpeech", request)
    }
    /// Synthesizes the requested text using the owner's local model engine.
    fn synthesizeLocalSpeech(
        &self,
        request: LocalTtsInferenceHostRequest,
    ) -> HostResult<LocalTtsInferenceHostResponse> {
        Self::execute("synthesizeLocalSpeech", request)
    }
}

/// Publishes session changes while delegating terminal work to the supplied host.
#[derive(Clone)]
pub(crate) struct RuntimeSessionPublishingTerminalHost {
    inner: Arc<dyn operit_host_api::TerminalHost>,
}

impl RuntimeSessionPublishingTerminalHost {
    /// Creates a terminal host wrapper that publishes runtime session snapshots.
    pub(crate) fn new(inner: Arc<dyn operit_host_api::TerminalHost>) -> Self {
        Self { inner }
    }

    /// Publishes the current terminal session snapshot through RuntimeTerminalService.
    fn publish_sessions(&self) -> operit_host_api::HostResult<()> {
        operit_runtime::services::publish_terminal_sessions_for_host(self)
            .map(|_| ())
            .map_err(operit_host_api::HostError::new)
    }
}

impl operit_host_api::TerminalHost for RuntimeSessionPublishingTerminalHost {
    /// Preserves the wrapped terminal's ownership of its working-directory namespace.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> operit_host_api::HostResult<String>,
    ) -> operit_host_api::HostResult<String> {
        self.inner.resolveWorkingDirectory(
            terminal,
            terminalType,
            workingDir,
            resolveHostDirectory,
        )
    }

    /// Returns terminal capabilities exposed by the wrapped host.
    fn terminalInfo(&self) -> operit_host_api::HostResult<operit_host_api::TerminalInfo> {
        self.inner.terminalInfo()
    }

    /// Starts a PTY session and publishes the updated session list.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> operit_host_api::HostResult<String> {
        let sessionId = self.inner.startPtySession(
            sessionName,
            terminal,
            terminalType,
            workingDir,
            rows,
            cols,
        )?;
        self.publish_sessions()?;
        Ok(sessionId)
    }

    /// Reads buffered PTY output from the wrapped host.
    fn readPtySession(&self, sessionId: &str) -> operit_host_api::HostResult<Vec<u8>> {
        self.inner.readPtySession(sessionId)
    }

    /// Writes bytes to a PTY session on the wrapped host.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> operit_host_api::HostResult<usize> {
        self.inner.writePtySession(sessionId, data)
    }

    /// Resizes a PTY session on the wrapped host.
    fn resizePtySession(
        &self,
        sessionId: &str,
        rows: u16,
        cols: u16,
    ) -> operit_host_api::HostResult<()> {
        self.inner.resizePtySession(sessionId, rows, cols)
    }

    /// Polls one PTY session exit state from the wrapped host.
    fn pollPtyExitCode(&self, sessionId: &str) -> operit_host_api::HostResult<Option<i32>> {
        self.inner.pollPtyExitCode(sessionId)
    }

    /// Closes a PTY session and publishes the updated session list.
    fn closePtySession(&self, sessionId: &str) -> operit_host_api::HostResult<()> {
        self.inner.closePtySession(sessionId)?;
        self.publish_sessions()
    }

    /// Lists terminal sessions from the wrapped host.
    fn listSessions(
        &self,
    ) -> operit_host_api::HostResult<Vec<operit_host_api::TerminalSessionListEntry>> {
        self.inner.listSessions()
    }

    /// Creates or returns a named terminal session and publishes the updated session list.
    fn createOrGetSession(
        &self,
        sessionName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalSessionInfo> {
        let session = self.inner.createOrGetSession(sessionName)?;
        self.publish_sessions()?;
        Ok(session)
    }

    /// Creates or returns an exactly typed session and publishes the updated session list.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalSessionInfo> {
        let session = self.inner.createOrGetTypedSession(sessionName, terminalType)?;
        self.publish_sessions()?;
        Ok(session)
    }

    /// Executes a command inside an existing terminal session.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalCommandOutput> {
        self.inner.executeInSession(sessionId, command, timeoutMs)
    }

    /// Executes a hidden command through the wrapped host.
    fn executeHiddenCommand(
        &self,
        command: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> operit_host_api::HostResult<operit_host_api::HiddenTerminalCommandOutput> {
        self.inner
            .executeHiddenCommand(command, executorKey, timeoutMs)
    }

    /// Sends text or control input to an existing terminal session.
    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalInputOutput> {
        self.inner.inputInSession(sessionId, input, control)
    }

    /// Closes a terminal session and publishes the updated session list.
    fn closeSession(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalCloseOutput> {
        let output = self.inner.closeSession(sessionId)?;
        self.publish_sessions()?;
        Ok(output)
    }

    /// Reads the current terminal screen from the wrapped host.
    fn getSessionScreen(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalScreenOutput> {
        self.inner.getSessionScreen(sessionId)
    }
}
