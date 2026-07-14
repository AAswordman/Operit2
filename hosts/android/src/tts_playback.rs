use std::sync::Arc;

use operit_host_api::{HostResult, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus};

#[derive(Clone, Debug)]
pub struct AndroidTtsPlaybackCommand {
    pub command: String,
    pub audioPath: Option<String>,
    pub request: Option<TtsPlaybackRequest>,
}

pub type AndroidTtsPlaybackController =
    Arc<dyn Fn(AndroidTtsPlaybackCommand) -> HostResult<TtsPlaybackStatus> + Send + Sync>;

#[derive(Clone)]
pub struct AndroidTtsPlaybackHost {
    controller: AndroidTtsPlaybackController,
}

impl AndroidTtsPlaybackHost {
    /// Creates an Android TTS host backed by the owner application.
    pub fn fromController(controller: AndroidTtsPlaybackController) -> Self {
        Self { controller }
    }

    /// Sends one playback command to the Android owner application.
    fn call(
        &self,
        command: &str,
        audioPath: Option<String>,
        request: Option<TtsPlaybackRequest>,
    ) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AndroidTtsPlaybackCommand {
            command: command.to_string(),
            audioPath,
            request,
        })
    }
}

impl TtsPlaybackHost for AndroidTtsPlaybackHost {
    /// Reports Android TextToSpeech availability through the owner host.
    fn supportsSystemSpeech(&self) -> bool {
        true
    }

    /// Starts one generated speech audio file in the Android owner application.
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
        self.call("play", Some(path.to_string()), None)
    }

    /// Starts one Android system speech request.
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        self.call("speak", None, Some(request))
    }

    /// Pauses the active Android speech session.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("pause", None, None)
    }

    /// Resumes the active Android speech session.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("resume", None, None)
    }

    /// Stops the active Android speech session.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("stop", None, None)
    }

    /// Returns the current Android speech session state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("state", None, None)
    }
}
