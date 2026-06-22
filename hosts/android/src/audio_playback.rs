use std::sync::Arc;

use operit_host_api::{AudioPlaybackHost, AudioPlaybackStatus, HostResult};

pub type AndroidAudioPlayer = Arc<dyn Fn(&str) -> HostResult<AudioPlaybackStatus> + Send + Sync>;

#[derive(Clone)]
pub struct AndroidAudioPlaybackHost {
    player: AndroidAudioPlayer,
}

impl AndroidAudioPlaybackHost {
    pub fn fromPlayer(player: AndroidAudioPlayer) -> Self {
        Self { player }
    }
}

impl AudioPlaybackHost for AndroidAudioPlaybackHost {
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        (self.player)(path)
    }
}
