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
    pub fn fromSynthesizer(synthesizer: AppleTtsSynthesizer) -> Self {
        Self { synthesizer }
    }
}

impl AppleTtsPlaybackHost {
    pub fn fromController(controller: AppleTtsPlaybackController) -> Self {
        Self { controller }
    }
}

impl TtsSynthesisHost for AppleTtsSynthesisHost {
    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> {
        (self.synthesizer)(request)
    }
}

impl TtsPlaybackHost for AppleTtsPlaybackHost {
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AppleTtsPlaybackCommand {
            command: "speak".to_string(),
            request: Some(request),
        })
    }

    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AppleTtsPlaybackCommand {
            command: "pause".to_string(),
            request: None,
        })
    }

    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AppleTtsPlaybackCommand {
            command: "resume".to_string(),
            request: None,
        })
    }

    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AppleTtsPlaybackCommand {
            command: "stop".to_string(),
            request: None,
        })
    }

    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AppleTtsPlaybackCommand {
            command: "status".to_string(),
            request: None,
        })
    }
}
