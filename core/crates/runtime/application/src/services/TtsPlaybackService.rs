#![allow(non_snake_case)]

use std::sync::Arc;

#[path = "ConfiguredTtsPlayback.rs"]
mod configured_playback;
use configured_playback::{cancel_configured_speech, replace_playback, start_configured_speech, SpeechStart};

use operit_host_api::{TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use operit_host_api::HostManager::HostManager;
use operit_model::TtsConfig::{
    TtsConfig, TtsHostPlaybackResult, TtsPlaybackResult, TtsProviderType,
};
use operit_util::TtsCleaner::TtsCleaner;

#[derive(Clone)]
/// Coordinates generated audio playback and host TTS playback controls.
pub struct TtsPlaybackService {
    filesystem: operit_vfs::VisualFileSystem::VisualFileSystem,
    ttsPlaybackHost: Option<Arc<dyn TtsPlaybackHost>>,
    ttsConfigManager: TtsConfigManager,
}

impl TtsPlaybackService {
    /// Creates a playback service from application host context.
    pub fn getInstance(context: &HostManager) -> Result<Self, String> {
        let paths = RuntimeStorePaths::default();
        Ok(Self {
            filesystem: context.vfs()?,
            ttsPlaybackHost: context.ttsPlaybackHost.clone(),
            ttsConfigManager: TtsConfigManager::new(paths),
        })
    }

    /// Plays a generated speech file through the TTS playback host.
    pub fn playAudio(&self, path: &str) -> Result<TtsPlaybackResult, String> {
        let path = path.trim();
        if path.is_empty() {
            return Err("audio path is empty".to_string());
        }
        let file = self.filesystem.fileResource(None, path)?;
        let host = self.ttsPlaybackHost()?;
        let status = replace_playback(&host, || {
            host.playFile(&file).map_err(|error| error.to_string())
        })?;
        Ok(TtsPlaybackResult {
            path: status.path,
            started: status.active,
            details: status.details,
        })
    }

    /// Speaks text with the exact configuration selected by a resolved chat profile.
    pub fn speakWithConfig(
        &self,
        ttsConfigId: &str,
        text: &str,
        interrupt: bool,
    ) -> Result<TtsHostPlaybackResult, String> {
        let ttsConfigId = ttsConfigId.trim();
        if ttsConfigId.is_empty() {
            return Err("tts config id is empty".to_string());
        }
        let cleanedText = TtsCleaner::clean(text);
        if cleanedText.is_empty() {
            return Err("tts text is empty".to_string());
        }
        let config = self
            .ttsConfigManager
            .getTtsConfig(ttsConfigId)
            .map_err(|error| error.to_string())?;
        self.speakWithResolvedConfig(config, cleanedText, interrupt, 1.0)
    }

    /// Starts system speech with one already resolved TTS configuration.
    pub(crate) fn speakWithResolvedConfig(
        &self,
        config: TtsConfig,
        cleanedText: String,
        interrupt: bool,
        pitch: f64,
    ) -> Result<TtsHostPlaybackResult, String> {
        validateSpeech(&cleanedText, config.speed, pitch)?;
        let providerType = TtsProviderType::normalize(&config.providerType);
        if providerType != TtsProviderType::SYSTEM_TTS {
            return Err(format!(
                "tts playback host only accepts SYSTEM_TTS config: {providerType}"
            ));
        }
        let host = self.ttsPlaybackHost()?;
        let request = systemSpeechRequest(config, cleanedText, interrupt, pitch);
        let speak = || host.speakText(request).map_err(|error| error.to_string());
        let status = if interrupt {
            replace_playback(&host, speak)?
        } else {
            speak()?
        };
        Ok(ttsHostPlaybackResult(status))
    }

    /// Starts configured speech using system voices or generated audio in order.
    pub(crate) async fn speakConfigured(
        context: &HostManager,
        config: TtsConfig,
        text: String,
        interrupt: bool,
        pitch: f64,
        waitForCompletion: bool,
        playbackKey: Option<(String, String)>,
    ) -> Result<TtsHostPlaybackResult, String> {
        let text = TtsCleaner::clean(&text);
        validateSpeech(&text, config.speed, pitch)?;
        let playback = Self::getInstance(context)?;
        let host = playback.ttsPlaybackHost()?;
        let system =
            TtsProviderType::normalize(&config.providerType) == TtsProviderType::SYSTEM_TTS;
        if !system && pitch != 1.0 {
            return Err("Pitch overrides are only supported by system TTS voices".into());
        }
        if system && !host.supportsSystemSpeech() {
            return Err("System TTS playback is not supported by this host".into());
        }
        let scheduler = context
            .hostRuntimeTaskSchedulerHost
            .clone()
            .ok_or("HostRuntimeTaskSchedulerHost is required for configured TTS playback")?;
        let synthesis = super::TtsSynthesisService::TtsSynthesisService::getInstance(context);
        let status = start_configured_speech(
            host,
            scheduler,
            interrupt,
            waitForCompletion,
            playbackKey,
            Box::new(move || {
                if system {
                    let request = systemSpeechRequest(config, text, interrupt, pitch);
                    return Ok(vec![Box::new(move |host: &dyn TtsPlaybackHost| {
                        host.speakText(request).map_err(|error| error.to_string())
                    }) as SpeechStart]);
                }
                let audio = synthesis.synthesizeWithResolvedConfig(&config.id, &config, &text)?;
                audio
                    .audioPaths
                    .into_iter()
                    .map(|path| {
                        let file = playback.filesystem.fileResource(None, &path)?;
                        Ok(Box::new(move |host: &dyn TtsPlaybackHost| {
                            host.playFile(&file).map_err(|error| error.to_string())
                        }) as SpeechStart)
                    })
                    .collect()
            }),
        )
        .await?;
        Ok(ttsHostPlaybackResult(status))
    }

    /// Cancels only the supplied plugin's configured utterance.
    pub(crate) fn cancelConfigured(&self, owner: &str, id: &str) -> Result<bool, String> {
        cancel_configured_speech(&self.ttsPlaybackHost()?, owner, id)
    }

    /// Pauses host speech playback.
    pub fn pauseSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .pauseSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    /// Resumes host speech playback.
    pub fn resumeSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .resumeSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    /// Stops host speech playback.
    pub fn stopSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        let host = self.ttsPlaybackHost()?;
        replace_playback(&host, || {
            host.stopSpeech().map_err(|error| error.to_string())
        })
        .map(ttsHostPlaybackResult)
    }

    /// Reads current host speech playback state.
    pub fn speechState(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .speechState()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    /// Returns the required platform TTS playback host.
    fn ttsPlaybackHost(&self) -> Result<Arc<dyn TtsPlaybackHost>, String> {
        self.ttsPlaybackHost
            .clone()
            .ok_or_else(|| "TtsPlaybackHost is required for TTS playback".to_string())
    }
}

/// Converts a host playback status into the Core proxy result model.
fn ttsHostPlaybackResult(status: TtsPlaybackStatus) -> TtsHostPlaybackResult {
    TtsHostPlaybackResult {
        path: status.path,
        active: status.active,
        paused: status.paused,
        details: status.details,
    }
}

/// Validates shared utterance controls before synthesis or playback changes host state.
fn validateSpeech(text: &str, speed: f64, pitch: f64) -> Result<(), String> {
    if text.is_empty() {
        return Err("tts text is empty".into());
    }
    if !speed.is_finite() || speed <= 0.0 {
        return Err("tts speech rate must be a finite positive number".into());
    }
    if !pitch.is_finite() || pitch <= 0.0 {
        return Err("tts pitch must be a finite positive number".into());
    }
    Ok(())
}

/// Converts one resolved system voice to the shared host request.
fn systemSpeechRequest(
    config: TtsConfig,
    text: String,
    interrupt: bool,
    pitch: f64,
) -> TtsPlaybackRequest {
    TtsPlaybackRequest {
        text,
        voice: config.voice,
        locale: config.model,
        speed: config.speed,
        pitch,
        interrupt,
    }
}
