use std::sync::Arc;

use operit_host_api::{
    AudioPlaybackHost, AudioPlaybackStatus, HostResult, MusicPlaybackRequest, MusicPlaybackStatus,
};

pub type OhosAudioPlayer = Arc<dyn Fn(&str) -> HostResult<AudioPlaybackStatus> + Send + Sync>;
pub type OhosMusicPlayer =
    Arc<dyn Fn(OhosMusicCommand) -> HostResult<MusicPlaybackStatus> + Send + Sync>;

pub enum OhosMusicCommand {
    Play(MusicPlaybackRequest),
    Pause,
    Resume,
    Stop,
    Seek(i64),
    SetVolume(f64),
    Status,
}

#[derive(Clone)]
pub struct OhosAudioPlaybackHost {
    player: OhosAudioPlayer,
    musicPlayer: OhosMusicPlayer,
}

impl OhosAudioPlaybackHost {
    /// Creates an OpenHarmony audio host from platform owner callbacks.
    pub fn fromPlayers(player: OhosAudioPlayer, musicPlayer: OhosMusicPlayer) -> Self {
        Self {
            player,
            musicPlayer,
        }
    }
}

impl AudioPlaybackHost for OhosAudioPlaybackHost {
    /// Plays a local audio file through the OpenHarmony owner app.
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        (self.player)(path)
    }

    /// Starts music playback through the OpenHarmony owner app.
    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(OhosMusicCommand::Play(request))
    }

    /// Pauses music playback through the OpenHarmony owner app.
    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(OhosMusicCommand::Pause)
    }

    /// Resumes music playback through the OpenHarmony owner app.
    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(OhosMusicCommand::Resume)
    }

    /// Stops music playback through the OpenHarmony owner app.
    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(OhosMusicCommand::Stop)
    }

    /// Seeks music playback through the OpenHarmony owner app.
    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(OhosMusicCommand::Seek(positionMs))
    }

    /// Updates music playback volume through the OpenHarmony owner app.
    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(OhosMusicCommand::SetVolume(volume))
    }

    /// Reads music playback status through the OpenHarmony owner app.
    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus> {
        (self.musicPlayer)(OhosMusicCommand::Status)
    }
}
