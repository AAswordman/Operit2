use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use operit_host_api::{
    HostError, HostResult, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus,
    TtsSynthesisHost, TtsSynthesisRequest, TtsSynthesisResponse,
};
use uuid::Uuid;
use windows_sys::Win32::Media::Multimedia::{mciGetErrorStringW, mciSendStringW};

#[derive(Clone, Debug, Default)]
pub struct WindowsTtsSynthesisHost;

impl WindowsTtsSynthesisHost {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowsTtsPlaybackHost {
    state: Arc<Mutex<WindowsTtsPlaybackState>>,
}

#[derive(Clone, Debug, Default)]
struct WindowsTtsPlaybackState {
    alias: Option<String>,
    path: String,
    paused: bool,
    details: String,
}

impl WindowsTtsPlaybackHost {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WindowsTtsPlaybackState {
                alias: None,
                path: String::new(),
                paused: false,
                details: "Windows system TTS idle".to_string(),
            })),
        }
    }
}

impl TtsPlaybackHost for WindowsTtsPlaybackHost {
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        let text = request.text.trim();
        if text.is_empty() {
            return Err(HostError::new("tts text is empty"));
        }
        {
            let mut state = self.state.lock().map_err(playbackLockError)?;
            if request.interrupt {
                stopWindowsPlayback(&mut state)?;
            } else if windowsPlaybackActive(&mut state)? {
                return Err(HostError::new("Windows system TTS playback is busy"));
            }
        }
        let synthesis = WindowsTtsSynthesisHost::new().synthesizeSpeech(TtsSynthesisRequest {
            text: text.to_string(),
            voice: request.voice,
            locale: request.locale,
            speed: request.speed,
            pitch: request.pitch,
            outputFormat: "wav".to_string(),
        })?;
        let alias = format!("operit_tts_{}", Uuid::new_v4().simple());
        let escapedPath = quotedMciPath(&synthesis.audioPath)?;
        mciCommand(&format!("open {escapedPath} alias {alias}"))?;
        let playResult = playOpenedAudio(&alias);
        if playResult.is_err() {
            let _ = mciCommand(&format!("close {alias}"));
        }
        playResult?;
        let mut state = self.state.lock().map_err(playbackLockError)?;
        state.alias = Some(alias);
        state.path = synthesis.audioPath;
        state.paused = false;
        state.details = "Windows system TTS playback started".to_string();
        Ok(windowsPlaybackStatus(&mut state)?)
    }

    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        let alias = state
            .alias
            .clone()
            .ok_or_else(|| HostError::new("Windows system TTS playback is not active"))?;
        mciCommand(&format!("pause {alias}"))?;
        state.paused = true;
        state.details = "Windows system TTS playback paused".to_string();
        windowsPlaybackStatus(&mut state)
    }

    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        let alias = state
            .alias
            .clone()
            .ok_or_else(|| HostError::new("Windows system TTS playback is not active"))?;
        mciCommand(&format!("resume {alias}"))?;
        state.paused = false;
        state.details = "Windows system TTS playback resumed".to_string();
        windowsPlaybackStatus(&mut state)
    }

    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        stopWindowsPlayback(&mut state)?;
        state.details = "Windows system TTS playback stopped".to_string();
        windowsPlaybackStatus(&mut state)
    }

    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        windowsPlaybackStatus(&mut state)
    }
}

impl TtsSynthesisHost for WindowsTtsSynthesisHost {
    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> {
        let text = request.text.trim();
        if text.is_empty() {
            return Err(HostError::new("tts text is empty"));
        }
        let outputFormat = request.outputFormat.trim();
        if outputFormat != "wav" {
            return Err(HostError::new(format!(
                "Windows system TTS only supports wav output: {outputFormat}"
            )));
        }
        let outputPath = ttsTempPath("wav")?;
        let scriptPath = ttsTempPath("ps1")?;
        let textPath = ttsTempPath("txt")?;
        let rate = windowsRate(request.speed)?;
        fs::write(&scriptPath, WINDOWS_TTS_SCRIPT.as_bytes()).map_err(|error| {
            HostError::new(format!("failed to write Windows system TTS script: {error}"))
        })?;
        fs::write(&textPath, text.as_bytes()).map_err(|error| {
            HostError::new(format!("failed to write Windows system TTS text: {error}"))
        })?;
        let statusResult = Command::new("powershell.exe")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(scriptPath.to_string_lossy().to_string())
            .arg("-TextPath")
            .arg(textPath.to_string_lossy().to_string())
            .arg("-Rate")
            .arg(rate.to_string())
            .arg("-OutputPath")
            .arg(outputPath.to_string_lossy().to_string())
            .arg("-Voice")
            .arg(request.voice.trim())
            .status()
            .map_err(|error| HostError::new(format!("Windows system TTS failed to start: {error}")));
        removeTempFile(&scriptPath)?;
        removeTempFile(&textPath)?;
        let status = statusResult?;
        if !status.success() {
            return Err(HostError::new(format!(
                "Windows system TTS failed with status: {status}"
            )));
        }
        if !outputPath.is_file() {
            return Err(HostError::new(format!(
                "Windows system TTS output missing: {}",
                outputPath.display()
            )));
        }
        Ok(TtsSynthesisResponse {
            audioPath: outputPath.to_string_lossy().to_string(),
            details: "Windows System.Speech synthesis completed".to_string(),
        })
    }
}

const WINDOWS_TTS_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory=$true)][string]$TextPath,
    [Parameter(Mandatory=$true)][int]$Rate,
    [Parameter(Mandatory=$true)][string]$OutputPath,
    [Parameter(Mandatory=$false)][string]$Voice = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$text = Get-Content -LiteralPath $TextPath -Raw -Encoding UTF8
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    if ($Voice.Length -gt 0) { $synth.SelectVoice($Voice) }
    $synth.Rate = $Rate
    $synth.SetOutputToWaveFile($OutputPath)
    $synth.Speak($text)
} finally {
    $synth.Dispose()
}
"#;

fn windowsPlaybackStatus(state: &mut WindowsTtsPlaybackState) -> HostResult<TtsPlaybackStatus> {
    let active = windowsPlaybackActive(state)?;
    Ok(TtsPlaybackStatus {
        path: state.path.clone(),
        active,
        paused: active && state.paused,
        details: state.details.clone(),
    })
}

fn windowsPlaybackActive(state: &mut WindowsTtsPlaybackState) -> HostResult<bool> {
    let Some(alias) = state.alias.clone() else {
        state.paused = false;
        return Ok(false);
    };
    let mode = mciCommandWithResponse(&format!("status {alias} mode"))?;
    match mode.trim() {
        "playing" => {
            state.paused = false;
            Ok(true)
        }
        "paused" => {
            state.paused = true;
            Ok(true)
        }
        "stopped" => {
            closeWindowsPlayback(state)?;
            state.details = "Windows system TTS playback completed".to_string();
            Ok(false)
        }
        value => Err(HostError::new(format!(
            "unexpected Windows system TTS playback mode: {value}"
        ))),
    }
}

fn stopWindowsPlayback(state: &mut WindowsTtsPlaybackState) -> HostResult<()> {
    if let Some(alias) = state.alias.clone() {
        mciCommand(&format!("stop {alias}"))?;
    }
    closeWindowsPlayback(state)
}

fn closeWindowsPlayback(state: &mut WindowsTtsPlaybackState) -> HostResult<()> {
    if let Some(alias) = state.alias.take() {
        mciCommand(&format!("close {alias}"))?;
    }
    if !state.path.is_empty() {
        removeTempFile(&PathBuf::from(&state.path))?;
    }
    state.path.clear();
    state.paused = false;
    Ok(())
}

fn playOpenedAudio(alias: &str) -> HostResult<u64> {
    mciCommand(&format!("set {alias} time format milliseconds"))?;
    let durationMillis = mciCommandWithResponse(&format!("status {alias} length"))?
        .trim()
        .parse::<u64>()
        .map_err(|error| HostError::new(format!("Failed to read Windows TTS duration: {error}")))?;
    mciCommand(&format!("play {alias}"))?;
    Ok(durationMillis)
}

fn quotedMciPath(path: &str) -> HostResult<String> {
    if path.chars().any(|value| value == '"') {
        return Err(HostError::new("tts path contains invalid quote character"));
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
    let succeeded =
        unsafe { mciGetErrorStringW(errorCode, buffer.as_mut_ptr(), buffer.len() as u32) };
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

fn ttsTempPath(extension: &str) -> HostResult<PathBuf> {
    let dir = std::env::temp_dir().join("operit_tts");
    fs::create_dir_all(&dir)
        .map_err(|error| HostError::new(format!("failed to create tts temp dir: {error}")))?;
    Ok(dir.join(format!("{}.{}", Uuid::new_v4(), extension)))
}

fn removeTempFile(path: &PathBuf) -> HostResult<()> {
    fs::remove_file(path).map_err(|error| {
        HostError::new(format!(
            "failed to remove Windows system TTS temp file {}: {error}",
            path.display()
        ))
    })
}

fn windowsRate(speed: f64) -> HostResult<i32> {
    if !speed.is_finite() || speed <= 0.0 {
        return Err(HostError::new("tts speed must be positive"));
    }
    let rate = ((speed - 1.0) * 10.0).round() as i32;
    Ok(rate.clamp(-10, 10))
}

fn playbackLockError(error: std::sync::PoisonError<std::sync::MutexGuard<'_, WindowsTtsPlaybackState>>) -> HostError {
    HostError::new(format!("Windows system TTS playback lock poisoned: {error}"))
}
