use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use operit_host_api::{
    AudioPlaybackHost, AudioPlaybackStatus, HostError, HostResult, MusicPlaybackRequest,
    MusicPlaybackStatus,
};

#[derive(Clone, Debug)]
pub struct LinuxAudioPlaybackHost {
    music: Arc<Mutex<Option<LinuxMusicSession>>>,
}

#[derive(Debug)]
struct LinuxMusicSession {
    child: Child,
    source: String,
    sourceType: String,
    title: Option<String>,
    artist: Option<String>,
    volume: f64,
    loopPlayback: bool,
    startedAt: Instant,
    startPositionMs: i64,
    pausedAtMs: Option<i64>,
}

impl LinuxAudioPlaybackHost {
    pub fn new() -> Self {
        Self {
            music: Arc::new(Mutex::new(None)),
        }
    }

    fn lockMusic(&self) -> HostResult<std::sync::MutexGuard<'_, Option<LinuxMusicSession>>> {
        self.music
            .lock()
            .map_err(|error| HostError::new(format!("Linux music state lock failed: {error}")))
    }

    fn startMusicProcess(request: &MusicPlaybackRequest, positionMs: i64) -> HostResult<Child> {
        let mut command = Command::new("ffplay");
        command
            .arg("-nodisp")
            .arg("-autoexit")
            .arg("-loglevel")
            .arg("quiet")
            .arg("-volume")
            .arg(format!("{}", (request.volume * 100.0).round() as i64));
        if request.loopPlayback {
            command.arg("-loop").arg("0");
        }
        if positionMs > 0 {
            command
                .arg("-ss")
                .arg(format!("{:.3}", positionMs as f64 / 1000.0));
        }
        command
            .arg(&request.source)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| HostError::new(format!("Linux ffplay music start failed: {error}")))
    }

    fn terminateMusicProcess(child: &mut Child) -> HostResult<()> {
        match child.try_wait()? {
            Some(_) => Ok(()),
            None => {
                child.kill()?;
                let _ = child.wait();
                Ok(())
            }
        }
    }

    fn currentPositionMs(session: &LinuxMusicSession) -> i64 {
        match session.pausedAtMs {
            Some(positionMs) => positionMs,
            None => {
                session.startPositionMs + i64::try_from(session.startedAt.elapsed().as_millis()).unwrap_or(i64::MAX)
            }
        }
    }

    fn idleStatus(message: impl Into<String>) -> MusicPlaybackStatus {
        MusicPlaybackStatus {
            state: "stopped".to_string(),
            source: None,
            sourceType: None,
            title: None,
            artist: None,
            durationMs: None,
            positionMs: 0,
            bufferedPositionMs: 0,
            volume: 1.0,
            loopPlayback: false,
            message: message.into(),
        }
    }

    fn statusForSession(
        session: &mut LinuxMusicSession,
        message: impl Into<String>,
    ) -> HostResult<MusicPlaybackStatus> {
        let processState = session.child.try_wait()?;
        let state = match (processState, session.pausedAtMs) {
            (Some(_), _) => "stopped",
            (None, Some(_)) => "paused",
            (None, None) => "playing",
        };
        let positionMs = Self::currentPositionMs(session);
        Ok(MusicPlaybackStatus {
            state: state.to_string(),
            source: Some(session.source.clone()),
            sourceType: Some(session.sourceType.clone()),
            title: session.title.clone(),
            artist: session.artist.clone(),
            durationMs: None,
            positionMs,
            bufferedPositionMs: positionMs,
            volume: session.volume,
            loopPlayback: session.loopPlayback,
            message: message.into(),
        })
    }
}

impl Default for LinuxAudioPlaybackHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlaybackHost for LinuxAudioPlaybackHost {
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        let path = path.trim();
        if path.is_empty() {
            return Err(HostError::new("audio path is empty"));
        }
        if !Path::new(path).is_file() {
            return Err(HostError::new(format!("audio file not found: {path}")));
        }
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|error| HostError::new(format!("Failed to start Linux audio playback: {error}")))?;
        Ok(AudioPlaybackStatus {
            path: path.to_string(),
            started: true,
            details: "Audio playback requested through xdg-open".to_string(),
        })
    }

    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus> {
        if request.source.trim().is_empty() {
            return Err(HostError::new("music source is empty"));
        }
        if request.sourceType == "path" && !Path::new(&request.source).is_file() {
            return Err(HostError::new(format!(
                "music file not found: {}",
                request.source
            )));
        }
        if !matches!(request.sourceType.as_str(), "path" | "url" | "uri") {
            return Err(HostError::new(format!(
                "Linux music source_type is unsupported: {}",
                request.sourceType
            )));
        }
        if !(0.0..=1.0).contains(&request.volume) {
            return Err(HostError::new("music volume must be between 0 and 1"));
        }
        if request.startPositionMs < 0 {
            return Err(HostError::new("music start_position_ms must be non-negative"));
        }
        let child = Self::startMusicProcess(&request, request.startPositionMs)?;
        let session = LinuxMusicSession {
            child,
            source: request.source,
            sourceType: request.sourceType,
            title: request.title,
            artist: request.artist,
            volume: request.volume,
            loopPlayback: request.loopPlayback,
            startedAt: Instant::now(),
            startPositionMs: request.startPositionMs,
            pausedAtMs: None,
        };
        let mut guard = self.lockMusic()?;
        if let Some(mut previous) = guard.take() {
            Self::terminateMusicProcess(&mut previous.child)?;
        }
        *guard = Some(session);
        let session = guard
            .as_mut()
            .expect("Linux music session must exist after insertion");
        Self::statusForSession(session, "Linux music playback started")
    }

    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus> {
        let mut guard = self.lockMusic()?;
        let Some(session) = guard.as_mut() else {
            return Ok(Self::idleStatus("Linux music playback is stopped"));
        };
        if session.pausedAtMs.is_none() {
            let positionMs = Self::currentPositionMs(session);
            Self::terminateMusicProcess(&mut session.child)?;
            session.pausedAtMs = Some(positionMs);
        }
        Self::statusForSession(session, "Linux music playback paused")
    }

    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus> {
        let mut guard = self.lockMusic()?;
        let Some(session) = guard.as_mut() else {
            return Ok(Self::idleStatus("Linux music playback is stopped"));
        };
        let Some(positionMs) = session.pausedAtMs.take() else {
            return Self::statusForSession(session, "Linux music playback is already playing");
        };
        let request = MusicPlaybackRequest {
            source: session.source.clone(),
            sourceType: session.sourceType.clone(),
            title: session.title.clone(),
            artist: session.artist.clone(),
            loopPlayback: session.loopPlayback,
            volume: session.volume,
            startPositionMs: positionMs,
        };
        session.child = Self::startMusicProcess(&request, positionMs)?;
        session.startedAt = Instant::now();
        session.startPositionMs = positionMs;
        Self::statusForSession(session, "Linux music playback resumed")
    }

    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus> {
        let mut guard = self.lockMusic()?;
        let Some(mut session) = guard.take() else {
            return Ok(Self::idleStatus("Linux music playback is stopped"));
        };
        Self::terminateMusicProcess(&mut session.child)?;
        Ok(Self::idleStatus("Linux music playback stopped"))
    }

    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus> {
        if positionMs < 0 {
            return Err(HostError::new("music position_ms must be non-negative"));
        }
        let mut guard = self.lockMusic()?;
        let Some(session) = guard.as_mut() else {
            return Ok(Self::idleStatus("Linux music playback is stopped"));
        };
        if session.pausedAtMs.is_some() {
            session.pausedAtMs = Some(positionMs);
            return Self::statusForSession(session, "Linux music playback seeked");
        }
        Self::terminateMusicProcess(&mut session.child)?;
        let request = MusicPlaybackRequest {
            source: session.source.clone(),
            sourceType: session.sourceType.clone(),
            title: session.title.clone(),
            artist: session.artist.clone(),
            loopPlayback: session.loopPlayback,
            volume: session.volume,
            startPositionMs: positionMs,
        };
        session.child = Self::startMusicProcess(&request, positionMs)?;
        session.startedAt = Instant::now();
        session.startPositionMs = positionMs;
        Self::statusForSession(session, "Linux music playback seeked")
    }

    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(HostError::new("music volume must be between 0 and 1"));
        }
        let mut guard = self.lockMusic()?;
        let Some(session) = guard.as_mut() else {
            return Ok(Self::idleStatus("Linux music playback is stopped"));
        };
        session.volume = volume;
        if session.pausedAtMs.is_none() {
            let positionMs = Self::currentPositionMs(session);
            Self::terminateMusicProcess(&mut session.child)?;
            let request = MusicPlaybackRequest {
                source: session.source.clone(),
                sourceType: session.sourceType.clone(),
                title: session.title.clone(),
                artist: session.artist.clone(),
                loopPlayback: session.loopPlayback,
                volume,
                startPositionMs: positionMs,
            };
            session.child = Self::startMusicProcess(&request, positionMs)?;
            session.startedAt = Instant::now();
            session.startPositionMs = positionMs;
        }
        Self::statusForSession(session, "Linux music volume updated")
    }

    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus> {
        let mut guard = self.lockMusic()?;
        let Some(session) = guard.as_mut() else {
            return Ok(Self::idleStatus("Linux music playback is stopped"));
        };
        Self::statusForSession(session, "Linux music playback status")
    }
}
