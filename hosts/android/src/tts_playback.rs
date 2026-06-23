use std::sync::Arc;

use operit_host_api::{HostResult, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus};

#[derive(Clone, Debug)]
pub struct AndroidTtsPlaybackCommand {
    pub command: String,
    pub request: Option<TtsPlaybackRequest>,
}

pub type AndroidTtsPlaybackController =
    Arc<dyn Fn(AndroidTtsPlaybackCommand) -> HostResult<TtsPlaybackStatus> + Send + Sync>;

#[derive(Clone)]
pub struct AndroidTtsPlaybackHost {
    controller: AndroidTtsPlaybackController,
}

impl AndroidTtsPlaybackHost {
    pub fn fromController(controller: AndroidTtsPlaybackController) -> Self {
        Self { controller }
    }

    fn call(&self, command: &str, request: Option<TtsPlaybackRequest>) -> HostResult<TtsPlaybackStatus> {
        (self.controller)(AndroidTtsPlaybackCommand {
            command: command.to_string(),
            request,
        })
    }
}

impl TtsPlaybackHost for AndroidTtsPlaybackHost {
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        self.call("speak", Some(request))
    }

    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("pause", None)
    }

    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("resume", None)
    }

    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("stop", None)
    }

    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        self.call("state", None)
    }
}
