use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use operit_host_api::{
    HostError, HostResult, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus,
    TtsSynthesisHost, TtsSynthesisRequest, TtsSynthesisResponse,
};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct LinuxTtsSynthesisHost;

impl LinuxTtsSynthesisHost {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Clone, Debug, Default)]
pub struct LinuxTtsPlaybackHost {
    state: Arc<Mutex<LinuxTtsPlaybackState>>,
}

#[derive(Debug, Default)]
struct LinuxTtsPlaybackState {
    child: Option<Child>,
    path: String,
    paused: bool,
    details: String,
}

impl LinuxTtsPlaybackHost {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LinuxTtsPlaybackState {
                child: None,
                path: String::new(),
                paused: false,
                details: "Linux system TTS idle".to_string(),
            })),
        }
    }
}

impl TtsPlaybackHost for LinuxTtsPlaybackHost {
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        let text = request.text.trim();
        if text.is_empty() {
            return Err(HostError::new("tts text is empty"));
        }
        {
            let mut state = self.state.lock().map_err(playbackLockError)?;
            if request.interrupt {
                stopLinuxPlayback(&mut state)?;
            } else if linuxPlaybackActive(&mut state)? {
                return Err(HostError::new("Linux system TTS playback is busy"));
            }
        }
        let synthesis = LinuxTtsSynthesisHost::new().synthesizeSpeech(TtsSynthesisRequest {
            text: text.to_string(),
            voice: request.voice,
            locale: request.locale,
            speed: request.speed,
            pitch: request.pitch,
            outputFormat: "wav".to_string(),
        })?;
        let child = Command::new("aplay")
            .arg(&synthesis.audioPath)
            .spawn()
            .map_err(|error| {
                HostError::new(format!("Linux system TTS playback failed to start: {error}"))
            })?;
        let mut state = self.state.lock().map_err(playbackLockError)?;
        state.child = Some(child);
        state.path = synthesis.audioPath;
        state.paused = false;
        state.details = "Linux system TTS playback started".to_string();
        linuxPlaybackStatus(&mut state)
    }

    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        let child = state
            .child
            .as_ref()
            .ok_or_else(|| HostError::new("Linux system TTS playback is not active"))?;
        signalProcess(child.id(), "-STOP")?;
        state.paused = true;
        state.details = "Linux system TTS playback paused".to_string();
        linuxPlaybackStatus(&mut state)
    }

    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        let child = state
            .child
            .as_ref()
            .ok_or_else(|| HostError::new("Linux system TTS playback is not active"))?;
        signalProcess(child.id(), "-CONT")?;
        state.paused = false;
        state.details = "Linux system TTS playback resumed".to_string();
        linuxPlaybackStatus(&mut state)
    }

    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        stopLinuxPlayback(&mut state)?;
        state.details = "Linux system TTS playback stopped".to_string();
        linuxPlaybackStatus(&mut state)
    }

    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        linuxPlaybackStatus(&mut state)
    }
}

impl TtsSynthesisHost for LinuxTtsSynthesisHost {
    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> {
        let text = request.text.trim();
        if text.is_empty() {
            return Err(HostError::new("tts text is empty"));
        }
        let outputFormat = request.outputFormat.trim();
        if outputFormat != "wav" {
            return Err(HostError::new(format!(
                "Linux system TTS only supports wav output: {outputFormat}"
            )));
        }
        let outputPath = ttsOutputPath("wav")?;
        let speed = linuxSpeed(request.speed)?;
        let status = Command::new("espeak-ng")
            .arg("-w")
            .arg(outputPath.to_string_lossy().to_string())
            .arg("-s")
            .arg(speed.to_string())
            .args(linuxVoiceArgs(request.voice.trim()))
            .arg(text)
            .status()
            .map_err(|error| HostError::new(format!("Linux system TTS failed to start: {error}")))?;
        if !status.success() {
            return Err(HostError::new(format!(
                "Linux system TTS failed with status: {status}"
            )));
        }
        if !outputPath.is_file() {
            return Err(HostError::new(format!(
                "Linux system TTS output missing: {}",
                outputPath.display()
            )));
        }
        Ok(TtsSynthesisResponse {
            audioPath: outputPath.to_string_lossy().to_string(),
            details: "Linux espeak-ng synthesis completed".to_string(),
        })
    }
}

fn ttsOutputPath(extension: &str) -> HostResult<PathBuf> {
    let dir = std::env::temp_dir().join("operit_tts");
    fs::create_dir_all(&dir)
        .map_err(|error| HostError::new(format!("failed to create tts temp dir: {error}")))?;
    Ok(dir.join(format!("{}.{}", Uuid::new_v4(), extension)))
}

fn linuxSpeed(speed: f64) -> HostResult<i32> {
    if !speed.is_finite() || speed <= 0.0 {
        return Err(HostError::new("tts speed must be positive"));
    }
    let wordsPerMinute = (175.0 * speed).round() as i32;
    Ok(wordsPerMinute.clamp(80, 450))
}

fn linuxVoiceArgs(voice: &str) -> Vec<String> {
    if voice.is_empty() {
        return Vec::new();
    }
    vec!["-v".to_string(), voice.to_string()]
}

fn linuxPlaybackStatus(state: &mut LinuxTtsPlaybackState) -> HostResult<TtsPlaybackStatus> {
    let active = linuxPlaybackActive(state)?;
    Ok(TtsPlaybackStatus {
        path: state.path.clone(),
        active,
        paused: active && state.paused,
        details: state.details.clone(),
    })
}

fn linuxPlaybackActive(state: &mut LinuxTtsPlaybackState) -> HostResult<bool> {
    let Some(child) = state.child.as_mut() else {
        state.paused = false;
        return Ok(false);
    };
    match child
        .try_wait()
        .map_err(|error| HostError::new(format!("Linux system TTS state check failed: {error}")))?
    {
        Some(_status) => {
            clearLinuxPlayback(state)?;
            state.details = "Linux system TTS playback completed".to_string();
            Ok(false)
        }
        None => Ok(true),
    }
}

fn stopLinuxPlayback(state: &mut LinuxTtsPlaybackState) -> HostResult<()> {
    if let Some(child) = state.child.as_mut() {
        child
            .kill()
            .map_err(|error| HostError::new(format!("Linux system TTS stop failed: {error}")))?;
        child
            .wait()
            .map_err(|error| HostError::new(format!("Linux system TTS wait failed: {error}")))?;
    }
    clearLinuxPlayback(state)
}

fn clearLinuxPlayback(state: &mut LinuxTtsPlaybackState) -> HostResult<()> {
    state.child = None;
    if !state.path.is_empty() {
        fs::remove_file(&state.path).map_err(|error| {
            HostError::new(format!(
                "failed to remove Linux system TTS temp file {}: {error}",
                state.path
            ))
        })?;
    }
    state.path.clear();
    state.paused = false;
    Ok(())
}

fn signalProcess(processId: u32, signal: &str) -> HostResult<()> {
    let status = Command::new("kill")
        .arg(signal)
        .arg(processId.to_string())
        .status()
        .map_err(|error| HostError::new(format!("Linux system TTS signal failed: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "Linux system TTS signal failed with status: {status}"
        )))
    }
}

fn playbackLockError(error: std::sync::PoisonError<std::sync::MutexGuard<'_, LinuxTtsPlaybackState>>) -> HostError {
    HostError::new(format!("Linux system TTS playback lock poisoned: {error}"))
}
