use std::sync::Arc;

use operit_host_api::{
    AudioPlaybackHost, AudioPlaybackStatus, HostResult, MusicPlaybackRequest, MusicPlaybackStatus,
};

pub type AppleAudioPlayer = Arc<dyn Fn(&str) -> HostResult<AudioPlaybackStatus> + Send + Sync>;
pub type AppleMusicPlayer =
    Arc<dyn Fn(AppleMusicCommand) -> HostResult<MusicPlaybackStatus> + Send + Sync>;

pub enum AppleMusicCommand {
    Play(MusicPlaybackRequest),
    Pause,
    Resume,
    Stop,
    Seek(i64),
    SetVolume(f64),
    Status,
}

#[derive(Clone)]
pub struct AppleAudioPlaybackHost {
    player: AppleAudioPlayer,
    musicPlayer: AppleMusicPlayer,
}

impl AppleAudioPlaybackHost {
    pub fn fromPlayers(player: AppleAudioPlayer, musicPlayer: AppleMusicPlayer) -> Self {
        Self {
            player,
            musicPlayer,
        }
    }
}

impl AudioPlaybackHost for AppleAudioPlaybackHost {
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        (self.player)(path)
    }

    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AppleMusicCommand::Play(request))
    }

    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AppleMusicCommand::Pause)
    }

    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AppleMusicCommand::Resume)
    }

    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AppleMusicCommand::Stop)
    }

    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AppleMusicCommand::Seek(positionMs))
    }

    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AppleMusicCommand::SetVolume(volume))
    }

    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AppleMusicCommand::Status)
    }
}
