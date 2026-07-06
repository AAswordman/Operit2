use std::sync::Arc;

use operit_host_api::{
    AudioPlaybackHost, AudioPlaybackStatus, HostResult, MusicPlaybackRequest, MusicPlaybackStatus,
};

pub type AndroidAudioPlayer = Arc<dyn Fn(&str) -> HostResult<AudioPlaybackStatus> + Send + Sync>;
pub type AndroidMusicPlayer =
    Arc<dyn Fn(AndroidMusicCommand) -> HostResult<MusicPlaybackStatus> + Send + Sync>;

pub enum AndroidMusicCommand {
    Play(MusicPlaybackRequest),
    Pause,
    Resume,
    Stop,
    Seek(i64),
    SetVolume(f64),
    Status,
}

#[derive(Clone)]
pub struct AndroidAudioPlaybackHost {
    player: AndroidAudioPlayer,
    musicPlayer: AndroidMusicPlayer,
}

impl AndroidAudioPlaybackHost {
    pub fn fromPlayers(player: AndroidAudioPlayer, musicPlayer: AndroidMusicPlayer) -> Self {
        Self {
            player,
            musicPlayer,
        }
    }
}

impl AudioPlaybackHost for AndroidAudioPlaybackHost {
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        (self.player)(path)
    }

    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AndroidMusicCommand::Play(request))
    }

    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AndroidMusicCommand::Pause)
    }

    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AndroidMusicCommand::Resume)
    }

    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AndroidMusicCommand::Stop)
    }

    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AndroidMusicCommand::Seek(positionMs))
    }

    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AndroidMusicCommand::SetVolume(volume))
    }

    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(AndroidMusicCommand::Status)
    }
}
