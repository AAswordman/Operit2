use std::path::Path;
use std::thread;
use std::time::Duration;

use operit_host_api::{AudioPlaybackHost, AudioPlaybackStatus, HostError, HostResult};
use uuid::Uuid;
use windows_sys::Win32::Media::Multimedia::{mciGetErrorStringW, mciSendStringW};

#[derive(Clone, Debug, Default)]
pub struct WindowsAudioPlaybackHost;

impl WindowsAudioPlaybackHost {
    pub fn new() -> Self {
        Self
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
