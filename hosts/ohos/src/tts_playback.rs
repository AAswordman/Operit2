use std::sync::Arc;

use operit_host_api::{
    HostError, HostResult, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus,
};

#[derive(Clone, Debug)]
pub struct OhosTtsPlaybackCommand {
    pub command: String,
    pub audioPath: Option<String>,
}

pub type OhosTtsPlaybackController =
    Arc<dyn Fn(OhosTtsPlaybackCommand) -> HostResult<TtsPlaybackStatus> + Send + Sync>;

#[derive(Clone)]
pub struct OhosTtsPlaybackHost {
    controller: OhosTtsPlaybackController,
}

impl OhosTtsPlaybackHost {
    /// Creates an OpenHarmony TTS playback host backed by the owner AVPlayer.
    pub fn new(controller: OhosTtsPlaybackController) -> Self {
        Self { controller }
    }

    /// Sends one playback command to the OpenHarmony owner application.
    fn call(&self, command: &str, audioPath: Option<String>) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(OhosTtsPlaybackCommand {
            command: command.to_string(),
            audioPath,
        })
    }
}

impl TtsPlaybackHost for OhosTtsPlaybackHost {
    /// Reports that OpenHarmony API 18 has no public system speech synthesis API.
    fn supportsSystemSpeech(&self) -> bool {
        false
    }

    /// Starts one generated speech audio file through OpenHarmony AVPlayer.
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
        let path = path.trim();
        if path.is_empty() {
            return Err(HostError::new("OHOS TTS audio path is empty"));
        }
        self.call("play", Some(path.to_string()))
    }

    /// Reports that OpenHarmony API 18 has no public system speech synthesis host.
    fn speakText(&self, _request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        Err(HostError::new(
            "OpenHarmony API 18 does not provide a system TTS synthesis host",
        ))
    }

    /// Pauses the active OpenHarmony TTS AVPlayer session.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("pause", None)
    }

    /// Resumes the active OpenHarmony TTS AVPlayer session.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("resume", None)
    }

    /// Stops the active OpenHarmony TTS AVPlayer session.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("stop", None)
    }

    /// Returns the current OpenHarmony TTS AVPlayer state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("state", None)
    }
}
