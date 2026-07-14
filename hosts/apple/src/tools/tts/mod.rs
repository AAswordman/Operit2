use std::sync::Arc;

use operit_host_api::{
    HostResult, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus, TtsSynthesisHost,
    TtsSynthesisRequest, TtsSynthesisResponse,
};

pub type AppleTtsSynthesizer =
    Arc<dyn Fn(TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> + Send + Sync>;
pub type AppleTtsPlaybackController =
    Arc<dyn Fn(AppleTtsPlaybackCommand) -> HostResult<TtsPlaybackStatus> + Send + Sync>;

pub struct AppleTtsPlaybackCommand {
    pub command: String,
    pub audioPath: Option<String>,
    pub request: Option<TtsPlaybackRequest>,
}

#[derive(Clone)]
pub struct AppleTtsSynthesisHost {
    synthesizer: AppleTtsSynthesizer,
}

#[derive(Clone)]
pub struct AppleTtsPlaybackHost {
    controller: AppleTtsPlaybackController,
}

impl AppleTtsSynthesisHost {
    /// Creates an Apple TTS synthesis host backed by the owner application.
    pub fn fromSynthesizer(synthesizer: AppleTtsSynthesizer) -> Self {
        Self { synthesizer }
    }
}

impl AppleTtsPlaybackHost {
    /// Creates an Apple TTS playback host backed by the owner application.
    pub fn fromController(controller: AppleTtsPlaybackController) -> Self {
        Self { controller }
    }

    /// Sends one playback command to the Apple owner application.
    fn call(
        &self,
        command: &str,
        audioPath: Option<String>,
        request: Option<TtsPlaybackRequest>,
    ) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AppleTtsPlaybackCommand {
            command: command.to_string(),
            audioPath,
            request,
        })
    }
}

impl TtsSynthesisHost for AppleTtsSynthesisHost {
    /// Synthesizes one Apple system speech request into an audio file.
    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> {
        (self.synthesizer)(request)
    }
}

impl TtsPlaybackHost for AppleTtsPlaybackHost {
    /// Reports Apple AVSpeechSynthesizer availability through the owner host.
    fn supportsSystemSpeech(&self) -> bool {
        true
    }

    /// Starts one generated speech audio file in the Apple owner application.
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
        self.call("play", Some(path.to_string()), None)
    }

    /// Starts one Apple system speech request.
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        self.call("speak", None, Some(request))
    }

    /// Pauses the active Apple speech session.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("pause", None, None)
    }

    /// Resumes the active Apple speech session.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("resume", None, None)
    }

    /// Stops the active Apple speech session.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("stop", None, None)
    }

    /// Returns the current Apple speech session state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("status", None, None)
    }
}
