use std::path::Path;
use std::process::Command;

use operit_host_api::{AudioPlaybackHost, AudioPlaybackStatus, HostError, HostResult};

#[derive(Clone, Debug, Default)]
pub struct LinuxAudioPlaybackHost;

impl LinuxAudioPlaybackHost {
    pub fn new() -> Self {
        Self
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
}
