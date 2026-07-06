use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use operit_host_api::{
    AudioPlaybackHost, AudioPlaybackStatus, HostError, HostResult, MusicPlaybackRequest,
    MusicPlaybackStatus,
};
use uuid::Uuid;
use windows_sys::Win32::Media::Multimedia::{mciGetErrorStringW, mciSendStringW};

#[derive(Clone, Debug)]
pub struct WindowsAudioPlaybackHost {
    music: Arc<Mutex<Option<WindowsMusicSession>>>,
}

#[derive(Clone, Debug)]
struct WindowsMusicSession {
    alias: String,
    source: String,
    sourceType: String,
    title: Option<String>,
    artist: Option<String>,
    volume: f64,
    loopPlayback: bool,
}

impl WindowsAudioPlaybackHost {
    pub fn new() -> Self {
        Self {
            music: Arc::new(Mutex::new(None)),
        }
    }

    fn lockMusic(&self) -> HostResult<std::sync::MutexGuard<'_, Option<WindowsMusicSession>>> {
        self.music
            .lock()
            .map_err(|error| HostError::new(format!("Windows music state lock failed: {error}")))
    }

    fn statusForSession(
        &self,
        session: &WindowsMusicSession,
        message: impl Into<String>,
    ) -> HostResult<MusicPlaybackStatus> {
        let state = mciCommandWithResponse(&format!("status {} mode", session.alias))?
            .trim()
            .to_string();
        let positionMs = mciCommandWithResponse(&format!("status {} position", session.alias))?
            .trim()
            .parse::<i64>()
            .map_err(|error| HostError::new(format!("Windows music position parse failed: {error}")))?;
        let durationMs = mciCommandWithResponse(&format!("status {} length", session.alias))?
            .trim()
            .parse::<i64>()
            .map_err(|error| HostError::new(format!("Windows music duration parse failed: {error}")))?;
        Ok(MusicPlaybackStatus {
            state,
            source: Some(session.source.clone()),
            sourceType: Some(session.sourceType.clone()),
            title: session.title.clone(),
            artist: session.artist.clone(),
            durationMs: Some(durationMs),
            positionMs,
            bufferedPositionMs: positionMs,
            volume: session.volume,
            loopPlayback: session.loopPlayback,
            message: message.into(),
        })
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

    fn closeMusicSession(session: WindowsMusicSession) {
        let _ = mciCommand(&format!("stop {}", session.alias));
        let _ = mciCommand(&format!("close {}", session.alias));
    }

    fn setMusicVolumeForAlias(alias: &str, volume: f64) -> HostResult<()> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(HostError::new("music volume must be between 0 and 1"));
        }
        let mciVolume = (volume * 1000.0).round() as i64;
        mciCommand(&format!("setaudio {alias} volume to {mciVolume}"))
    }
}

impl Default for WindowsAudioPlaybackHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlaybackHost for WindowsAudioPlaybackHost {
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        let path = path.trim();
        if path.is_empty() {
            return Err(HostError::new("audio path is empty"));
        }
        if !Path::new(path).is_file() {
            return Err(HostError::new(format!("audio file not found: {path}")));
        }
        let alias = format!("operit_audio_{}", Uuid::new_v4().simple());
        let escapedPath = quotedMciPath(path)?;
        mciCommand(&format!("open {escapedPath} alias {alias}"))?;
        let playResult = playOpenedAudio(&alias);
        if playResult.is_err() {
            let _ = mciCommand(&format!("close {alias}"));
        }
        let durationMillis = playResult?;
        let cleanupAlias = alias.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(durationMillis.saturating_add(500)));
            let _ = mciCommand(&format!("close {cleanupAlias}"));
        });
        Ok(AudioPlaybackStatus {
            path: path.to_string(),
            started: true,
            details: "Audio playback started through Windows MCI".to_string(),
        })
    }

    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus> {
        if request.source.trim().is_empty() {
            return Err(HostError::new("music source is empty"));
        }
        if request.sourceType != "path" {
            return Err(HostError::new(format!(
                "Windows music source_type is unsupported: {}",
                request.sourceType
            )));
        }
        if !Path::new(&request.source).is_file() {
            return Err(HostError::new(format!(
                "music file not found: {}",
                request.source
            )));
        }
        let alias = format!("operit_music_{}", Uuid::new_v4().simple());
        let escapedPath = quotedMciPath(&request.source)?;
        mciCommand(&format!("open {escapedPath} alias {alias}"))?;
        let openedAlias = alias.clone();
        let result = (|| {
            mciCommand(&format!("set {alias} time format milliseconds"))?;
            Self::setMusicVolumeForAlias(&alias, request.volume)?;
            let playCommand = if request.loopPlayback {
                format!("play {alias} from {} repeat", request.startPositionMs)
            } else {
                format!("play {alias} from {}", request.startPositionMs)
            };
            mciCommand(&playCommand)?;
            Ok(())
        })();
        if let Err(error) = result {
            let _ = mciCommand(&format!("close {openedAlias}"));
            return Err(error);
        }
        let session = WindowsMusicSession {
            alias,
            source: request.source,
            sourceType: request.sourceType,
            title: request.title,
            artist: request.artist,
            volume: request.volume,
            loopPlayback: request.loopPlayback,
        };
        {
            let mut guard = self.lockMusic()?;
            if let Some(previous) = guard.take() {
                Self::closeMusicSession(previous);
            }
            *guard = Some(session.clone());
        }
        self.statusForSession(&session, "Windows music playback started")
    }

    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus> {
        let guard = self.lockMusic()?;
        let Some(session) = guard.as_ref() else {
            return Ok(Self::idleStatus("Windows music playback is stopped"));
        };
        mciCommand(&format!("pause {}", session.alias))?;
        self.statusForSession(session, "Windows music playback paused")
    }

    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus> {
        let guard = self.lockMusic()?;
        let Some(session) = guard.as_ref() else {
            return Ok(Self::idleStatus("Windows music playback is stopped"));
        };
        mciCommand(&format!("resume {}", session.alias))?;
        self.statusForSession(session, "Windows music playback resumed")
    }

    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus> {
        let mut guard = self.lockMusic()?;
        let Some(session) = guard.take() else {
            return Ok(Self::idleStatus("Windows music playback is stopped"));
        };
        Self::closeMusicSession(session);
        Ok(Self::idleStatus("Windows music playback stopped"))
    }

    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus> {
        if positionMs < 0 {
            return Err(HostError::new("music position_ms must be non-negative"));
        }
        let guard = self.lockMusic()?;
        let Some(session) = guard.as_ref() else {
            return Ok(Self::idleStatus("Windows music playback is stopped"));
        };
        let mode = mciCommandWithResponse(&format!("status {} mode", session.alias))?
            .trim()
            .to_string();
        if mode == "playing" {
            let playCommand = if session.loopPlayback {
                format!("play {} from {positionMs} repeat", session.alias)
            } else {
                format!("play {} from {positionMs}", session.alias)
            };
            mciCommand(&playCommand)?;
        } else {
            mciCommand(&format!("seek {} to {positionMs}", session.alias))?;
        }
        self.statusForSession(session, "Windows music playback seeked")
    }

    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus> {
        let mut guard = self.lockMusic()?;
        let Some(session) = guard.as_mut() else {
            return Ok(Self::idleStatus("Windows music playback is stopped"));
        };
        Self::setMusicVolumeForAlias(&session.alias, volume)?;
        session.volume = volume;
        self.statusForSession(session, "Windows music volume updated")
    }

    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus> {
        let guard = self.lockMusic()?;
        let Some(session) = guard.as_ref() else {
            return Ok(Self::idleStatus("Windows music playback is stopped"));
        };
        self.statusForSession(session, "Windows music playback status")
    }
}

fn playOpenedAudio(alias: &str) -> HostResult<u64> {
    mciCommand(&format!("set {alias} time format milliseconds"))?;
    let durationMillis = mciCommandWithResponse(&format!("status {alias} length"))?
        .trim()
        .parse::<u64>()
        .map_err(|error| HostError::new(format!("Failed to read Windows audio duration: {error}")))?;
    mciCommand(&format!("play {alias}"))?;
    Ok(durationMillis)
}

fn quotedMciPath(path: &str) -> HostResult<String> {
    if path.chars().any(|value| value == '"') {
        return Err(HostError::new("audio path contains invalid quote character"));
    }
    Ok(format!("\"{path}\""))
}

fn mciCommand(command: &str) -> HostResult<()> {
    let errorCode = unsafe {
        mciSendStringW(
            wideString(command).as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        )
    };
    if errorCode == 0 {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "Windows MCI command failed: {}",
            mciErrorMessage(errorCode)
        )))
    }
}

fn mciCommandWithResponse(command: &str) -> HostResult<String> {
    let mut buffer = vec![0u16; 256];
    let errorCode = unsafe {
        mciSendStringW(
            wideString(command).as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            std::ptr::null_mut(),
        )
    };
    if errorCode != 0 {
        return Err(HostError::new(format!(
            "Windows MCI command failed: {}",
            mciErrorMessage(errorCode)
        )));
    }
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..end]))
}

fn mciErrorMessage(errorCode: u32) -> String {
    let mut buffer = vec![0u16; 512];
    let succeeded = unsafe {
        mciGetErrorStringW(errorCode, buffer.as_mut_ptr(), buffer.len() as u32)
    };
    if succeeded == 0 {
        return format!("MCI error code {errorCode}");
    }
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn wideString(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
