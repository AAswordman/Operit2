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
    /// Creates a stateless Linux TTS synthesizer.
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
    ownedPath: bool,
    paused: bool,
    preparing: bool,
    generation: u64,
    details: String,
}

impl LinuxTtsPlaybackHost {
    /// Creates an idle Linux TTS playback host.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LinuxTtsPlaybackState {
                child: None,
                path: String::new(),
                ownedPath: false,
                paused: false,
                preparing: false,
                generation: 0,
                details: "Linux system TTS idle".to_string(),
            })),
        }
    }
}

impl TtsPlaybackHost for LinuxTtsPlaybackHost {
    /// Reports Linux espeak-ng system speech availability.
    fn supportsSystemSpeech(&self) -> bool {
        true
    }

    /// Starts one generated speech audio file through aplay.
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
        let path = path.trim();
        if path.is_empty() {
            return Err(HostError::new("tts audio path is empty"));
        }
        if !PathBuf::from(path).is_file() {
            return Err(HostError::new(format!(
                "tts audio file does not exist: {path}"
            )));
        }
        let mut state = self.state.lock().map_err(playbackLockError)?;
        state.generation = state.generation.wrapping_add(1);
        state.preparing = false;
        stopLinuxPlayback(&mut state)?;
        startLinuxAudio(
            &mut state,
            path.to_string(),
            false,
            "Linux generated TTS playback started",
        )
    }

    /// Synthesizes and starts one generation-checked Linux utterance.
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        let text = request.text.trim();
        if text.is_empty() {
            return Err(HostError::new("tts text is empty"));
        }
        let generation = {
            let mut state = self.state.lock().map_err(playbackLockError)?;
            if request.interrupt {
                stopLinuxPlayback(&mut state)?;
            } else if state.preparing || linuxPlaybackActive(&mut state)? {
                return Err(HostError::new("Linux system TTS playback is busy"));
            }
            state.generation = state.generation.wrapping_add(1);
            state.preparing = true;
            state.details = "Linux system TTS synthesis started".to_string();
            state.generation
        };
        let synthesis = match LinuxTtsSynthesisHost::new().synthesizeSpeech(TtsSynthesisRequest {
            text: text.to_string(),
            voice: request.voice,
            locale: request.locale,
            speed: request.speed,
            pitch: request.pitch,
            outputFormat: "wav".to_string(),
        }) {
            Ok(synthesis) => synthesis,
            Err(error) => {
                let mut state = self.state.lock().map_err(playbackLockError)?;
                if state.generation == generation {
                    state.preparing = false;
                    state.details = "Linux system TTS synthesis failed".to_string();
                }
                return Err(error);
            }
        };
        let mut state = self.state.lock().map_err(playbackLockError)?;
        if state.generation != generation {
            fs::remove_file(&synthesis.audioPath).map_err(|error| {
                HostError::new(format!(
                    "failed to remove cancelled Linux TTS file {}: {error}",
                    synthesis.audioPath
                ))
            })?;
            return linuxPlaybackStatus(&mut state);
        }
        startLinuxAudio(
            &mut state,
            synthesis.audioPath,
            true,
            "Linux system TTS playback started",
        )
    }

    /// Pauses the active aplay process.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        if !linuxPlaybackActive(&mut state)? {
            state.details = "Linux system TTS playback is not active".to_string();
            return linuxPlaybackStatus(&mut state);
        }
        if state.paused {
            state.details = "Linux system TTS playback is already paused".to_string();
            return linuxPlaybackStatus(&mut state);
        }
        let Some(child) = state.child.as_ref() else {
            state.details = "Linux system TTS playback is not active".to_string();
            return linuxPlaybackStatus(&mut state);
        };
        signalProcess(child.id(), "-STOP")?;
        state.paused = true;
        state.details = "Linux system TTS playback paused".to_string();
        linuxPlaybackStatus(&mut state)
    }

    /// Resumes the paused aplay process.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        if !linuxPlaybackActive(&mut state)? {
            state.details = "Linux system TTS playback is not active".to_string();
            return linuxPlaybackStatus(&mut state);
        }
        if !state.paused {
            state.details = "Linux system TTS playback is not paused".to_string();
            return linuxPlaybackStatus(&mut state);
        }
        let Some(child) = state.child.as_ref() else {
            state.details = "Linux system TTS playback is not active".to_string();
            return linuxPlaybackStatus(&mut state);
        };
        signalProcess(child.id(), "-CONT")?;
        state.paused = false;
        state.details = "Linux system TTS playback resumed".to_string();
        linuxPlaybackStatus(&mut state)
    }

    /// Invalidates preparation and stops active aplay playback.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        state.generation = state.generation.wrapping_add(1);
        state.preparing = false;
        stopLinuxPlayback(&mut state)?;
        state.details = "Linux system TTS playback stopped".to_string();
        linuxPlaybackStatus(&mut state)
    }

    /// Returns the current Linux speech playback state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        linuxPlaybackStatus(&mut state)
    }
}

impl TtsSynthesisHost for LinuxTtsSynthesisHost {
    /// Synthesizes one utterance into a temporary WAV file.
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
        let pitch = linuxPitch(request.pitch)?;
        let status = Command::new("espeak-ng")
            .arg("-w")
            .arg(outputPath.to_string_lossy().to_string())
            .arg("-s")
            .arg(speed.to_string())
            .arg("-p")
            .arg(pitch.to_string())
            .args(linuxVoiceArgs(request.voice.trim(), request.locale.trim())?)
            .arg(text)
            .status()
            .map_err(|error| {
                HostError::new(format!("Linux system TTS failed to start: {error}"))
            })?;
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

/// Creates a unique Linux TTS output path.
fn ttsOutputPath(extension: &str) -> HostResult<PathBuf> {
    let dir = std::env::temp_dir().join("operit_tts");
    fs::create_dir_all(&dir)
        .map_err(|error| HostError::new(format!("failed to create tts temp dir: {error}")))?;
    Ok(dir.join(format!("{}.{}", Uuid::new_v4(), extension)))
}

/// Converts the common speed multiplier into words per minute.
fn linuxSpeed(speed: f64) -> HostResult<i32> {
    if !speed.is_finite() || speed <= 0.0 {
        return Err(HostError::new("tts speed must be positive"));
    }
    let wordsPerMinute = (175.0 * speed).round() as i32;
    if !(80..=450).contains(&wordsPerMinute) {
        return Err(HostError::new(
            "tts speed is outside the espeak-ng rate range",
        ));
    }
    Ok(wordsPerMinute)
}

/// Converts the common pitch multiplier into the espeak-ng pitch range.
fn linuxPitch(pitch: f64) -> HostResult<i32> {
    if !pitch.is_finite() || pitch <= 0.0 {
        return Err(HostError::new("tts pitch must be positive"));
    }
    let value = (50.0 * pitch).round() as i32;
    if !(0..=99).contains(&value) {
        return Err(HostError::new(
            "tts pitch is outside the espeak-ng pitch range",
        ));
    }
    Ok(value)
}

/// Builds explicit espeak-ng voice arguments from the requested voice or locale.
fn linuxVoiceArgs(voice: &str, locale: &str) -> HostResult<Vec<String>> {
    match (voice.is_empty(), locale.is_empty()) {
        (true, true) => Ok(Vec::new()),
        (false, true) => Ok(vec!["-v".to_string(), voice.to_string()]),
        (true, false) => Ok(vec!["-v".to_string(), locale.to_string()]),
        (false, false) => Err(HostError::new(
            "espeak-ng accepts either an explicit voice or a locale, not both",
        )),
    }
}

/// Builds a Linux status snapshot and reconciles natural completion.
fn linuxPlaybackStatus(state: &mut LinuxTtsPlaybackState) -> HostResult<TtsPlaybackStatus> {
    let active = linuxPlaybackActive(state)?;
    Ok(TtsPlaybackStatus {
        path: state.path.clone(),
        active,
        paused: active && state.paused,
        details: state.details.clone(),
    })
}

/// Checks whether the tracked aplay child remains active.
fn linuxPlaybackActive(state: &mut LinuxTtsPlaybackState) -> HostResult<bool> {
    let Some(child) = state.child.as_mut() else {
        state.paused = false;
        return Ok(false);
    };
    match child
        .try_wait()
        .map_err(|error| HostError::new(format!("Linux system TTS state check failed: {error}")))?
    {
        Some(status) => {
            clearLinuxPlayback(state)?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "Linux system TTS playback failed with status: {status}"
                )));
            }
            state.details = "Linux system TTS playback completed".to_string();
            Ok(false)
        }
        None => Ok(true),
    }
}

/// Stops and reaps the tracked aplay child.
fn stopLinuxPlayback(state: &mut LinuxTtsPlaybackState) -> HostResult<()> {
    if let Some(child) = state.child.as_mut() {
        if child
            .try_wait()
            .map_err(|error| {
                HostError::new(format!("Linux system TTS state check failed: {error}"))
            })?
            .is_some()
        {
            return clearLinuxPlayback(state);
        }
        child
            .kill()
            .map_err(|error| HostError::new(format!("Linux system TTS stop failed: {error}")))?;
        child
            .wait()
            .map_err(|error| HostError::new(format!("Linux system TTS wait failed: {error}")))?;
    }
    clearLinuxPlayback(state)
}

/// Clears Linux playback state and removes its temporary file.
fn clearLinuxPlayback(state: &mut LinuxTtsPlaybackState) -> HostResult<()> {
    state.child = None;
    if state.ownedPath && !state.path.is_empty() {
        fs::remove_file(&state.path).map_err(|error| {
            HostError::new(format!(
                "failed to remove Linux system TTS temp file {}: {error}",
                state.path
            ))
        })?;
    }
    state.path.clear();
    state.ownedPath = false;
    state.paused = false;
    Ok(())
}

/// Starts one aplay process for a speech audio file.
fn startLinuxAudio(
    state: &mut LinuxTtsPlaybackState,
    path: String,
    ownedPath: bool,
    details: &str,
) -> HostResult<TtsPlaybackStatus> {
    state.preparing = false;
    let child = match Command::new("aplay").arg(&path).spawn() {
        Ok(value) => value,
        Err(error) => {
            if ownedPath {
                fs::remove_file(&path).map_err(|removeError| {
                    HostError::new(format!(
                        "failed to remove Linux TTS file {path} after start error: {removeError}"
                    ))
                })?;
            }
            return Err(HostError::new(format!(
                "Linux TTS playback failed to start: {error}"
            )));
        }
    };
    state.child = Some(child);
    state.path = path;
    state.ownedPath = ownedPath;
    state.paused = false;
    state.details = details.to_string();
    linuxPlaybackStatus(state)
}

/// Sends a POSIX signal to the tracked playback process.
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

/// Converts a poisoned playback lock into a host error.
fn playbackLockError(
    error: std::sync::PoisonError<std::sync::MutexGuard<'_, LinuxTtsPlaybackState>>,
) -> HostError {
    HostError::new(format!("Linux system TTS playback lock poisoned: {error}"))
}
