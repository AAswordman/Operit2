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
    /// Creates a stateless Windows TTS synthesizer.
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
    ownedPath: bool,
    paused: bool,
    preparing: bool,
    generation: u64,
    details: String,
}

impl WindowsTtsPlaybackHost {
    /// Creates an idle Windows TTS playback host.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WindowsTtsPlaybackState {
                alias: None,
                path: String::new(),
                ownedPath: false,
                paused: false,
                preparing: false,
                generation: 0,
                details: "Windows system TTS idle".to_string(),
            })),
        }
    }
}

impl TtsPlaybackHost for WindowsTtsPlaybackHost {
    /// Reports Windows System.Speech availability.
    fn supportsSystemSpeech(&self) -> bool {
        true
    }

    /// Starts one generated speech audio file through Windows MCI.
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
        stopWindowsPlayback(&mut state)?;
        startWindowsAudio(
            &mut state,
            path.to_string(),
            false,
            "Windows generated TTS playback started",
        )
    }

    /// Synthesizes and starts one generation-checked Windows utterance.
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        let text = request.text.trim();
        if text.is_empty() {
            return Err(HostError::new("tts text is empty"));
        }
        let generation = {
            let mut state = self.state.lock().map_err(playbackLockError)?;
            if request.interrupt {
                stopWindowsPlayback(&mut state)?;
            } else if state.preparing || windowsPlaybackActive(&mut state)? {
                return Err(HostError::new("Windows system TTS playback is busy"));
            }
            state.generation = state.generation.wrapping_add(1);
            state.preparing = true;
            state.details = "Windows system TTS synthesis started".to_string();
            state.generation
        };
        let synthesis = match WindowsTtsSynthesisHost::new().synthesizeSpeech(TtsSynthesisRequest {
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
                    state.details = "Windows system TTS synthesis failed".to_string();
                }
                return Err(error);
            }
        };
        let mut state = self.state.lock().map_err(playbackLockError)?;
        if state.generation != generation {
            removeTempFile(&PathBuf::from(&synthesis.audioPath))?;
            return windowsPlaybackStatus(&mut state);
        }
        startWindowsAudio(
            &mut state,
            synthesis.audioPath,
            true,
            "Windows system TTS playback started",
        )
    }

    /// Pauses active MCI playback.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        if !windowsPlaybackActive(&mut state)? {
            state.details = "Windows system TTS playback is not active".to_string();
            return windowsPlaybackStatus(&mut state);
        }
        if state.paused {
            state.details = "Windows system TTS playback is already paused".to_string();
            return windowsPlaybackStatus(&mut state);
        }
        let Some(alias) = state.alias.clone() else {
            state.details = "Windows system TTS playback is not active".to_string();
            return windowsPlaybackStatus(&mut state);
        };
        mciCommand(&format!("pause {alias}"))?;
        state.paused = true;
        state.details = "Windows system TTS playback paused".to_string();
        windowsPlaybackStatus(&mut state)
    }

    /// Resumes paused MCI playback.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        if !windowsPlaybackActive(&mut state)? {
            state.details = "Windows system TTS playback is not active".to_string();
            return windowsPlaybackStatus(&mut state);
        }
        if !state.paused {
            state.details = "Windows system TTS playback is not paused".to_string();
            return windowsPlaybackStatus(&mut state);
        }
        let Some(alias) = state.alias.clone() else {
            state.details = "Windows system TTS playback is not active".to_string();
            return windowsPlaybackStatus(&mut state);
        };
        mciCommand(&format!("resume {alias}"))?;
        state.paused = false;
        state.details = "Windows system TTS playback resumed".to_string();
        windowsPlaybackStatus(&mut state)
    }

    /// Invalidates preparation and stops active MCI playback.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        state.generation = state.generation.wrapping_add(1);
        state.preparing = false;
        stopWindowsPlayback(&mut state)?;
        state.details = "Windows system TTS playback stopped".to_string();
        windowsPlaybackStatus(&mut state)
    }

    /// Returns the current Windows speech playback state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        let mut state = self.state.lock().map_err(playbackLockError)?;
        windowsPlaybackStatus(&mut state)
    }
}

impl TtsSynthesisHost for WindowsTtsSynthesisHost {
    /// Synthesizes one utterance into a temporary WAV file.
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
        validateWindowsPitch(request.pitch)?;
        fs::write(&scriptPath, WINDOWS_TTS_SCRIPT.as_bytes()).map_err(|error| {
            HostError::new(format!(
                "failed to write Windows system TTS script: {error}"
            ))
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
            .arg("-Locale")
            .arg(request.locale.trim())
            .status()
            .map_err(|error| {
                HostError::new(format!("Windows system TTS failed to start: {error}"))
            });
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
    [Parameter(Mandatory=$false)][string]$Voice = '',
    [Parameter(Mandatory=$false)][string]$Locale = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$text = Get-Content -LiteralPath $TextPath -Raw -Encoding UTF8
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    if ($Voice.Length -gt 0) { $synth.SelectVoice($Voice) }
    if ($Locale.Length -gt 0) {
        $culture = [System.Globalization.CultureInfo]::GetCultureInfo($Locale)
        if ($Voice.Length -gt 0) {
            if ($synth.Voice.Culture.Name -ne $culture.Name) {
                throw "Voice locale $($synth.Voice.Culture.Name) does not match requested locale $($culture.Name)"
            }
        } else {
            $synth.SelectVoiceByHints(
                [System.Speech.Synthesis.VoiceGender]::NotSet,
                [System.Speech.Synthesis.VoiceAge]::NotSet,
                0,
                $culture
            )
        }
    }
    $synth.Rate = $Rate
    $synth.SetOutputToWaveFile($OutputPath)
    $synth.Speak($text)
} finally {
    $synth.Dispose()
}
"#;

/// Builds a status snapshot and reconciles natural completion.
fn windowsPlaybackStatus(state: &mut WindowsTtsPlaybackState) -> HostResult<TtsPlaybackStatus> {
    let active = windowsPlaybackActive(state)?;
    Ok(TtsPlaybackStatus {
        path: state.path.clone(),
        active,
        paused: active && state.paused,
        details: state.details.clone(),
    })
}

/// Checks whether the tracked MCI device is active.
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

/// Stops and closes the tracked MCI device.
fn stopWindowsPlayback(state: &mut WindowsTtsPlaybackState) -> HostResult<()> {
    if !windowsPlaybackActive(state)? {
        return Ok(());
    }
    if let Some(alias) = state.alias.clone() {
        mciCommand(&format!("stop {alias}"))?;
    }
    closeWindowsPlayback(state)
}

/// Closes MCI playback and removes its temporary file.
fn closeWindowsPlayback(state: &mut WindowsTtsPlaybackState) -> HostResult<()> {
    if let Some(alias) = state.alias.take() {
        mciCommand(&format!("close {alias}"))?;
    }
    if state.ownedPath && !state.path.is_empty() {
        removeTempFile(&PathBuf::from(&state.path))?;
    }
    state.path.clear();
    state.ownedPath = false;
    state.paused = false;
    Ok(())
}

/// Opens and starts one Windows MCI speech audio file.
fn startWindowsAudio(
    state: &mut WindowsTtsPlaybackState,
    path: String,
    ownedPath: bool,
    details: &str,
) -> HostResult<TtsPlaybackStatus> {
    state.preparing = false;
    let alias = format!("operit_tts_{}", Uuid::new_v4().simple());
    let escapedPath = match quotedMciPath(&path) {
        Ok(value) => value,
        Err(error) => {
            if ownedPath {
                removeTempFile(&PathBuf::from(&path))?;
            }
            return Err(error);
        }
    };
    if let Err(error) = mciCommand(&format!("open {escapedPath} alias {alias}")) {
        if ownedPath {
            removeTempFile(&PathBuf::from(&path))?;
        }
        return Err(error);
    }
    if let Err(error) = playOpenedAudio(&alias) {
        mciCommand(&format!("close {alias}"))?;
        if ownedPath {
            removeTempFile(&PathBuf::from(&path))?;
        }
        return Err(error);
    }
    state.alias = Some(alias);
    state.path = path;
    state.ownedPath = ownedPath;
    state.paused = false;
    state.details = details.to_string();
    windowsPlaybackStatus(state)
}

/// Starts an opened MCI audio alias.
fn playOpenedAudio(alias: &str) -> HostResult<u64> {
    mciCommand(&format!("set {alias} time format milliseconds"))?;
    let durationMillis = mciCommandWithResponse(&format!("status {alias} length"))?
        .trim()
        .parse::<u64>()
        .map_err(|error| HostError::new(format!("Failed to read Windows TTS duration: {error}")))?;
    mciCommand(&format!("play {alias}"))?;
    Ok(durationMillis)
}

/// Quotes a validated path for an MCI command.
fn quotedMciPath(path: &str) -> HostResult<String> {
    if path.chars().any(|value| value == '"') {
        return Err(HostError::new("tts path contains invalid quote character"));
    }
    Ok(format!("\"{path}\""))
}

/// Executes an MCI command without a response payload.
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

/// Executes an MCI command and reads its textual response.
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

/// Resolves a Windows MCI error code.
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

/// Encodes a null-terminated Windows UTF-16 string.
fn wideString(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Creates a unique path in the Windows TTS temporary directory.
fn ttsTempPath(extension: &str) -> HostResult<PathBuf> {
    let dir = std::env::temp_dir().join("operit_tts");
    fs::create_dir_all(&dir)
        .map_err(|error| HostError::new(format!("failed to create tts temp dir: {error}")))?;
    Ok(dir.join(format!("{}.{}", Uuid::new_v4(), extension)))
}

/// Removes a Windows TTS temporary file.
fn removeTempFile(path: &PathBuf) -> HostResult<()> {
    fs::remove_file(path).map_err(|error| {
        HostError::new(format!(
            "failed to remove Windows system TTS temp file {}: {error}",
            path.display()
        ))
    })
}

/// Converts the common speed multiplier into a System.Speech rate.
fn windowsRate(speed: f64) -> HostResult<i32> {
    if !speed.is_finite() || speed <= 0.0 {
        return Err(HostError::new("tts speed must be positive"));
    }
    let rate = ((speed - 1.0) * 10.0).round() as i32;
    if !(-10..=10).contains(&rate) {
        return Err(HostError::new(
            "tts speed is outside the Windows speech rate range",
        ));
    }
    Ok(rate)
}

/// Rejects pitch values that System.Speech cannot apply without altering input semantics.
fn validateWindowsPitch(pitch: f64) -> HostResult<()> {
    if !pitch.is_finite() || pitch <= 0.0 {
        return Err(HostError::new("tts pitch must be positive"));
    }
    if pitch != 1.0 {
        return Err(HostError::new(
            "Windows System.Speech playback only supports pitch 1.0",
        ));
    }
    Ok(())
}

/// Converts a poisoned playback lock into a host error.
fn playbackLockError(
    error: std::sync::PoisonError<std::sync::MutexGuard<'_, WindowsTtsPlaybackState>>,
) -> HostError {
    HostError::new(format!(
        "Windows system TTS playback lock poisoned: {error}"
    ))
}
