use std::fs;
use std::path::PathBuf;
use std::process::Command;

use operit_host_api::{HostError, HostResult, TtsSynthesisHost, TtsSynthesisRequest, TtsSynthesisResponse};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct WindowsTtsSynthesisHost;

impl WindowsTtsSynthesisHost {
    pub fn new() -> Self {
        Self
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
