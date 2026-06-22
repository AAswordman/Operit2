use std::fs;
use std::path::PathBuf;
use std::process::Command;

use operit_host_api::{HostError, HostResult, TtsSynthesisHost, TtsSynthesisRequest, TtsSynthesisResponse};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct LinuxTtsSynthesisHost;

impl LinuxTtsSynthesisHost {
    pub fn new() -> Self {
        Self
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
