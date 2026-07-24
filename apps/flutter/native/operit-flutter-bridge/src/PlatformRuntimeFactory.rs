use super::*;

#[cfg(target_os = "android")]
#[derive(Clone)]
struct FlutterSystemOperationBridge {
    native: NativeSystemOperationHost,
}

#[cfg(target_os = "android")]
impl FlutterSystemOperationBridge {
    fn new() -> Self {
        Self {
            native: NativeSystemOperationHost::new(),
        }
    }
}

#[cfg(target_os = "android")]
impl operit_host_api::SystemOperationHost for FlutterSystemOperationBridge {
    fn getSystemLanguageCode(&self) -> operit_host_api::HostResult<String> {
        self.native.getSystemLanguageCode()
    }

    fn toast(&self, message: &str) -> operit_host_api::HostResult<()> {
        self.native.toast(message)
    }

    fn sendNotification(&self, title: &str, message: &str) -> operit_host_api::HostResult<()> {
        self.native.sendNotification(title, message)
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> {
        self.native.modifySystemSetting(namespace, setting, value)
    }

    fn getSystemSetting(
        &self,
        namespace: &str,
        setting: &str,
    ) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> {
        self.native.getSystemSetting(namespace, setting)
    }

    fn installApp(
        &self,
        path: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.installApp(path)
    }

    fn uninstallApp(
        &self,
        packageName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.uninstallApp(packageName)
    }

    fn listInstalledApps(
        &self,
        includeSystemApps: bool,
    ) -> operit_host_api::HostResult<operit_host_api::AppListData> {
        self.native.listInstalledApps(includeSystemApps)
    }

    fn startApp(
        &self,
        packageName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.startApp(packageName)
    }

    fn stopApp(
        &self,
        packageName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.stopApp(packageName)
    }

    fn getNotifications(
        &self,
        limit: i32,
        includeOngoing: bool,
    ) -> operit_host_api::HostResult<operit_host_api::NotificationData> {
        self.native.getNotifications(limit, includeOngoing)
    }

    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> operit_host_api::HostResult<operit_host_api::AppUsageTimeResultData> {
        self.native
            .getAppUsageTime(packageName, sinceHours, limit, includeSystemApps)
    }

    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> operit_host_api::HostResult<operit_host_api::LocationData> {
        self.native
            .getDeviceLocation(timeout, highAccuracy, includeAddress)
    }

    fn getDeviceInfo(&self) -> operit_host_api::HostResult<operit_host_api::DeviceInfoData> {
        self.native.getDeviceInfo()
    }

    fn captureScreenshot(&self) -> operit_host_api::HostResult<String> {
        let response = requestOwnerSystemCaptureScreenshot(Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        Ok(response.path)
    }

    fn recognizeText(
        &self,
        imagePath: &str,
        language: operit_host_api::OCRLanguage,
        quality: operit_host_api::OCRQuality,
    ) -> operit_host_api::HostResult<String> {
        let request = RuntimeHostInteractionSystemRecognizeTextPayload {
            imagePath: imagePath.to_string(),
            language: language.asHostValue().to_string(),
            quality: quality.asHostValue().to_string(),
        };
        let response = requestOwnerSystemRecognizeText(request, Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        Ok(response.text)
    }
}

#[cfg(any(
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_env = "ohos"
))]
/// Creates a music playback command payload with default optional fields.
fn musicCommandPayload(command: &str) -> RuntimeHostInteractionMusicPlaybackPayload {
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

/// Serializes owner-command parameters into the JSON string expected by the platform channel.
fn serialize_owner_params_json(
    params: &serde_json::Value,
    label: &str,
) -> operit_host_api::HostResult<String> {
    serde_json::to_string(params).map_err(|error| {
        operit_host_api::HostError::new(format!("{label} params JSON encode failed: {error}"))
    })
}

#[cfg(any(
    windows,
    all(target_os = "linux", not(target_env = "ohos")),
    target_os = "android",
    target_os = "macos"
))]
pub(crate) fn create_local_core(
    runtime_root: PathBuf,
    workspace_root: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    let mut context =
        create_platform_runtime_host_manager(runtime_root, workspace_root, webVisitHost);
    #[cfg(target_os = "android")]
    {
        context.systemOperationHost = Some(Arc::new(FlutterSystemOperationBridge::new()));
    }
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = browserSessionHost {
        context = context.withBrowserSessionHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    context = context.withTerminalHost(terminalHost);
    #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
    {
        context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::fromPlayers(
            Arc::new(|path| {
                let response = requestOwnerAudioPlay(
                    RuntimeHostInteractionAudioPlayPayload {
                        path: path.to_string(),
                    },
                    Duration::from_secs(60),
                )
                .map_err(operit_host_api::HostError::new)?;
                Ok(operit_host_api::AudioPlaybackStatus {
                    path: response.path,
                    started: response.started,
                    details: response.details,
                })
            }),
            Arc::new(|command| {
                let payload = match command {
                    NativeMusicCommand::Play(request) => {
                        RuntimeHostInteractionMusicPlaybackPayload {
                            command: "play".to_string(),
                            source: Some(request.source),
                            sourceType: Some(request.sourceType),
                            title: request.title,
                            artist: request.artist,
                            loopPlayback: request.loopPlayback,
                            volume: request.volume,
                            positionMs: request.startPositionMs,
                        }
                    }
                    NativeMusicCommand::Pause => musicCommandPayload("pause"),
                    NativeMusicCommand::Resume => musicCommandPayload("resume"),
                    NativeMusicCommand::Stop => musicCommandPayload("stop"),
                    NativeMusicCommand::Status => musicCommandPayload("status"),
                    NativeMusicCommand::Seek(positionMs) => {
                        RuntimeHostInteractionMusicPlaybackPayload {
                            positionMs,
                            ..musicCommandPayload("seek")
                        }
                    }
                    NativeMusicCommand::SetVolume(volume) => {
                        RuntimeHostInteractionMusicPlaybackPayload {
                            volume,
                            ..musicCommandPayload("set_volume")
                        }
                    }
                };
                let response = requestOwnerMusicPlayback(payload, Duration::from_secs(60))
                    .map_err(operit_host_api::HostError::new)?;
                Ok(operit_host_api::MusicPlaybackStatus {
                    state: response.state,
                    source: response.source,
                    sourceType: response.sourceType,
                    title: response.title,
                    artist: response.artist,
                    durationMs: response.durationMs,
                    positionMs: response.positionMs,
                    bufferedPositionMs: response.bufferedPositionMs,
                    volume: response.volume,
                    loopPlayback: response.loopPlayback,
                    message: response.message,
                })
            }),
        )));
        context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::fromController(
            Arc::new(|command, params| {
                let response = requestOwnerBluetooth(
                    RuntimeHostInteractionBluetoothPayload {
                        command: command.to_string(),
                        paramsJson: serialize_owner_params_json(&params, "platform Bluetooth")?,
                    },
                    Duration::from_secs(120),
                )
                .map_err(operit_host_api::HostError::new)?;
                serde_json::from_str(&response.resultJson).map_err(|error| {
                    operit_host_api::HostError::new(format!(
                        "platform Bluetooth response JSON decode failed: {error}"
                    ))
                })
            }),
        )));
        #[cfg(target_os = "android")]
        {
            context = context.withTtsSynthesisHost(Arc::new(
                NativeTtsSynthesisHost::fromSynthesizer(Arc::new(|request| {
                    let response = requestOwnerTtsSynthesis(
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
                    .map_err(operit_host_api::HostError::new)?;
                    Ok(operit_host_api::TtsSynthesisResponse {
                        audioPath: response.audioPath,
                        details: response.details,
                    })
                })),
            ));
        }
        context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::fromController(
            Arc::new(|command| {
                let payload = match command.command.as_str() {
                    "play" => {
                        let audioPath = command.audioPath.ok_or_else(|| {
                            operit_host_api::HostError::new("tts play audio path is required")
                        })?;
                        RuntimeHostInteractionTtsPlaybackPayload {
                            command: command.command,
                            audioPath: Some(audioPath),
                            text: String::new(),
                            voice: String::new(),
                            locale: String::new(),
                            speed: 1.0,
                            pitch: 1.0,
                            interrupt: true,
                        }
                    }
                    "speak" => {
                        let request = match command.request {
                            Some(request) => request,
                            None => {
                                return Err(operit_host_api::HostError::new(
                                    "tts speak request is required",
                                ));
                            }
                        };
                        RuntimeHostInteractionTtsPlaybackPayload {
                            command: command.command,
                            audioPath: None,
                            text: request.text,
                            voice: request.voice,
                            locale: request.locale,
                            speed: request.speed,
                            pitch: request.pitch,
                            interrupt: request.interrupt,
                        }
                    }
                    "pause" | "resume" | "stop" | "state" | "status" => {
                        RuntimeHostInteractionTtsPlaybackPayload {
                            command: command.command,
                            audioPath: None,
                            text: String::new(),
                            voice: String::new(),
                            locale: String::new(),
                            speed: 1.0,
                            pitch: 1.0,
                            interrupt: false,
                        }
                    }
                    other => {
                        return Err(operit_host_api::HostError::new(format!(
                            "unsupported tts playback command: {other}"
                        )));
                    }
                };
                let response = requestOwnerTtsPlayback(payload, Duration::from_secs(120))
                    .map_err(operit_host_api::HostError::new)?;
                Ok(operit_host_api::TtsPlaybackStatus {
                    path: response.path,
                    active: response.active,
                    paused: response.paused,
                    details: response.details,
                })
            }),
        )));
    }
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(target_env = "ohos")]
/// Creates the OpenHarmony runtime context from explicit app storage roots.
pub(crate) fn create_local_core(
    runtime_root: PathBuf,
    workspace_root: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    let systemLanguageCode = Arc::new(systemLanguageCode);
    let mut context = create_platform_runtime_host_manager(
        runtime_root,
        workspace_root,
        webVisitHost,
        Arc::new(|path| {
            let response = requestOwnerFileOpen(
                RuntimeHostInteractionFileOpenPayload {
                    path: path.to_string(),
                },
                Duration::from_secs(60),
            )
            .map_err(operit_host_api::HostError::new)?;
            if response.success {
                return Ok(());
            }
            let Some(error) = response.error else {
                return Err(operit_host_api::HostError::new(
                    "file open error is missing",
                ));
            };
            Err(operit_host_api::HostError::new(error))
        }),
        Arc::new(|path, title| {
            let response = requestOwnerFileShare(
                RuntimeHostInteractionFileSharePayload {
                    path: path.to_string(),
                    title: title.to_string(),
                },
                Duration::from_secs(60),
            )
            .map_err(operit_host_api::HostError::new)?;
            if response.success {
                return Ok(());
            }
            let Some(error) = response.error else {
                return Err(operit_host_api::HostError::new(
                    "file share error is missing",
                ));
            };
            Err(operit_host_api::HostError::new(error))
        }),
        Arc::new(move || Ok(systemLanguageCode.as_ref().clone())),
        Arc::new(|| {
            let response = requestOwnerSystemCaptureScreenshot(Duration::from_secs(60))
                .map_err(operit_host_api::HostError::new)?;
            Ok(response.path)
        }),
        Arc::new(|imagePath, language, quality| {
            Err(operit_host_api::HostError::new(format!(
                "OpenHarmony OCR is unavailable in the configured SDK; imagePath={imagePath}, language={}, quality={}",
                language.asHostValue(),
                quality.asHostValue()
            )))
        }),
        Arc::new(|operation, params| {
            let response = requestOwnerSystemOperation(
                RuntimeHostInteractionSystemOperationPayload {
                    operation: operation.to_string(),
                    paramsJson: serialize_owner_params_json(
                        &params,
                        "OpenHarmony system operation",
                    )?,
                },
                Duration::from_secs(120),
            )
            .map_err(operit_host_api::HostError::new)?;
            serde_json::from_str(&response.resultJson).map_err(|error| {
                operit_host_api::HostError::new(format!(
                    "OpenHarmony system operation response JSON decode failed: {error}"
                ))
            })
        }),
    );
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = browserSessionHost {
        context = context.withBrowserSessionHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    context = context.withTerminalHost(terminalHost);
    context =
        context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::new(Arc::new(|command| {
            let payload = RuntimeHostInteractionTtsPlaybackPayload {
                command: command.command,
                audioPath: command.audioPath,
                text: String::new(),
                voice: String::new(),
                locale: String::new(),
                speed: 1.0,
                pitch: 1.0,
                interrupt: false,
            };
            let response = requestOwnerTtsPlayback(payload, Duration::from_secs(120))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::TtsPlaybackStatus {
                path: response.path,
                active: response.active,
                paused: response.paused,
                details: response.details,
            })
        }))));
    context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::fromPlayers(
        Arc::new(|path| {
            let response = requestOwnerAudioPlay(
                RuntimeHostInteractionAudioPlayPayload {
                    path: path.to_string(),
                },
                Duration::from_secs(60),
            )
            .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::AudioPlaybackStatus {
                path: response.path,
                started: response.started,
                details: response.details,
            })
        }),
        Arc::new(|command| {
            let payload = match command {
                NativeMusicCommand::Play(request) => RuntimeHostInteractionMusicPlaybackPayload {
                    command: "play".to_string(),
                    source: Some(request.source),
                    sourceType: Some(request.sourceType),
                    title: request.title,
                    artist: request.artist,
                    loopPlayback: request.loopPlayback,
                    volume: request.volume,
                    positionMs: request.startPositionMs,
                },
                NativeMusicCommand::Pause => musicCommandPayload("pause"),
                NativeMusicCommand::Resume => musicCommandPayload("resume"),
                NativeMusicCommand::Stop => musicCommandPayload("stop"),
                NativeMusicCommand::Status => musicCommandPayload("status"),
                NativeMusicCommand::Seek(positionMs) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        positionMs,
                        ..musicCommandPayload("seek")
                    }
                }
                NativeMusicCommand::SetVolume(volume) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        volume,
                        ..musicCommandPayload("set_volume")
                    }
                }
            };
            let response = requestOwnerMusicPlayback(payload, Duration::from_secs(60))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::MusicPlaybackStatus {
                state: response.state,
                source: response.source,
                sourceType: response.sourceType,
                title: response.title,
                artist: response.artist,
                durationMs: response.durationMs,
                positionMs: response.positionMs,
                bufferedPositionMs: response.bufferedPositionMs,
                volume: response.volume,
                loopPlayback: response.loopPlayback,
                message: response.message,
            })
        }),
    )));
    context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::fromController(Arc::new(
        |command, params| {
            let response = requestOwnerBluetooth(
                RuntimeHostInteractionBluetoothPayload {
                    command: command.to_string(),
                    paramsJson: serialize_owner_params_json(&params, "OpenHarmony Bluetooth")?,
                },
                Duration::from_secs(120),
            )
            .map_err(operit_host_api::HostError::new)?;
            serde_json::from_str(&response.resultJson).map_err(|error| {
                operit_host_api::HostError::new(format!(
                    "OpenHarmony Bluetooth response JSON decode failed: {error}"
                ))
            })
        },
    ))));
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(any(
    windows,
    all(target_os = "linux", not(target_env = "ohos")),
    target_os = "ios",
    target_os = "macos"
))]
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        NativeRuntimeStorageHost::defaultRuntimeRoot(),
        NativeRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

#[cfg(target_env = "ohos")]
/// Requires OpenHarmony owner code to provide application storage roots.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Err(
        "OpenHarmony runtime and workspace roots must be provided by the OpenHarmony host"
            .to_string(),
    )
}

#[cfg(target_os = "android")]
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Err("Android runtime and workspace roots must be provided by the Android host".to_string())
}

#[cfg(target_arch = "wasm32")]
/// Returns the storage roots owned by the browser runtime Host.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        WebRuntimeStorageHost::defaultRuntimeRoot(),
        WebRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

#[cfg(target_os = "ios")]
pub(crate) fn create_local_core(
    runtime_root: PathBuf,
    workspace_root: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    let mut context =
        create_platform_runtime_host_manager(runtime_root, workspace_root, webVisitHost);
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = browserSessionHost {
        context = context.withBrowserSessionHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    context = context.withTerminalHost(terminalHost);
    context = context.withLocalInferenceHost(Arc::new(NativeLocalInferenceHost::fromExecutor(
        Arc::new(|command| {
            let response = requestOwnerLocalInference(
                RuntimeHostInteractionLocalInferencePayload {
                    method: command.method,
                    requestJson: command.requestJson,
                },
                Duration::from_secs(600),
            )
            .map_err(operit_host_api::HostError::new)?;
            Ok(response.resultJson)
        }),
    )));
    context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::fromPlayers(
        Arc::new(|path| {
            let response = requestOwnerAudioPlay(
                RuntimeHostInteractionAudioPlayPayload {
                    path: path.to_string(),
                },
                Duration::from_secs(60),
            )
            .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::AudioPlaybackStatus {
                path: response.path,
                started: response.started,
                details: response.details,
            })
        }),
        Arc::new(|command| {
            let payload = match command {
                NativeMusicCommand::Play(request) => RuntimeHostInteractionMusicPlaybackPayload {
                    command: "play".to_string(),
                    source: Some(request.source),
                    sourceType: Some(request.sourceType),
                    title: request.title,
                    artist: request.artist,
                    loopPlayback: request.loopPlayback,
                    volume: request.volume,
                    positionMs: request.startPositionMs,
                },
                NativeMusicCommand::Pause => musicCommandPayload("pause"),
                NativeMusicCommand::Resume => musicCommandPayload("resume"),
                NativeMusicCommand::Stop => musicCommandPayload("stop"),
                NativeMusicCommand::Status => musicCommandPayload("status"),
                NativeMusicCommand::Seek(positionMs) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        positionMs,
                        ..musicCommandPayload("seek")
                    }
                }
                NativeMusicCommand::SetVolume(volume) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        volume,
                        ..musicCommandPayload("set_volume")
                    }
                }
            };
            let response = requestOwnerMusicPlayback(payload, Duration::from_secs(60))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::MusicPlaybackStatus {
                state: response.state,
                source: response.source,
                sourceType: response.sourceType,
                title: response.title,
                artist: response.artist,
                durationMs: response.durationMs,
                positionMs: response.positionMs,
                bufferedPositionMs: response.bufferedPositionMs,
                volume: response.volume,
                loopPlayback: response.loopPlayback,
                message: response.message,
            })
        }),
    )));
    context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::fromController(Arc::new(
        |command, params| {
            let response = requestOwnerBluetooth(
                RuntimeHostInteractionBluetoothPayload {
                    command: command.to_string(),
                    paramsJson: serialize_owner_params_json(&params, "platform Bluetooth")?,
                },
                Duration::from_secs(120),
            )
            .map_err(operit_host_api::HostError::new)?;
            serde_json::from_str(&response.resultJson).map_err(|error| {
                operit_host_api::HostError::new(format!(
                    "platform Bluetooth response JSON decode failed: {error}"
                ))
            })
        },
    ))));
    context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::fromController(
        Arc::new(|command| {
            let payload = match command.command.as_str() {
                "play" => {
                    let audioPath = command.audioPath.ok_or_else(|| {
                        operit_host_api::HostError::new("tts play audio path is required")
                    })?;
                    RuntimeHostInteractionTtsPlaybackPayload {
                        command: command.command,
                        audioPath: Some(audioPath),
                        text: String::new(),
                        voice: String::new(),
                        locale: String::new(),
                        speed: 1.0,
                        pitch: 1.0,
                        interrupt: true,
                    }
                }
                "speak" => {
                    let request = match command.request {
                        Some(request) => request,
                        None => {
                            return Err(operit_host_api::HostError::new(
                                "tts speak request is required",
                            ));
                        }
                    };
                    RuntimeHostInteractionTtsPlaybackPayload {
                        command: command.command,
                        audioPath: None,
                        text: request.text,
                        voice: request.voice,
                        locale: request.locale,
                        speed: request.speed,
                        pitch: request.pitch,
                        interrupt: request.interrupt,
                    }
                }
                "pause" | "resume" | "stop" | "status" => {
                    RuntimeHostInteractionTtsPlaybackPayload {
                        command: command.command,
                        audioPath: None,
                        text: String::new(),
                        voice: String::new(),
                        locale: String::new(),
                        speed: 1.0,
                        pitch: 1.0,
                        interrupt: false,
                    }
                }
                other => {
                    return Err(operit_host_api::HostError::new(format!(
                        "unsupported tts playback command: {other}"
                    )));
                }
            };
            let response = requestOwnerTtsPlayback(payload, Duration::from_secs(120))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::TtsPlaybackStatus {
                path: response.path,
                active: response.active,
                paused: response.paused,
                details: response.details,
            })
        }),
    )));
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(not(any(
    windows,
    all(target_os = "linux", not(target_env = "ohos")),
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_env = "ohos",
    target_arch = "wasm32"
)))]
pub(crate) fn create_local_core(
    _runtime_root: PathBuf,
    _workspace_root: PathBuf,
    _webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    _browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    _browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    _composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    #[cfg(any(
        windows,
        all(target_os = "linux", not(target_env = "ohos")),
        target_os = "android"
    ))]
    _terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    Err("operit flutter native runtime bridge is not available for this target".to_string())
}
