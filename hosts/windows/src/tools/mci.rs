use operit_host_api::{HostError, HostResult};
use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost;
use windows_sys::Win32::Media::Multimedia::{mciGetErrorStringW, mciSendStringW};

/// Keeps MCI aliases on the same reusable platform thread for their entire lifetime.
fn execute(command: &str, response: bool) -> HostResult<String> {
    let command = command.to_owned();
    NativeHostRuntimeTaskSchedulerHost::new().runPlatformTask("windows-mci", move || {
        let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
        let mut buffer = [0u16; 256];
        let error = unsafe {
            mciSendStringW(
                wide.as_ptr(),
                if response {
                    buffer.as_mut_ptr()
                } else {
                    std::ptr::null_mut()
                },
                if response { buffer.len() as u32 } else { 0 },
                std::ptr::null_mut(),
            )
        };
        if error != 0 {
            let mut message = [0u16; 512];
            let succeeded =
                unsafe { mciGetErrorStringW(error, message.as_mut_ptr(), message.len() as u32) };
            let operation = command.split_whitespace().next().unwrap_or("unknown");
            let details = if succeeded == 0 {
                format!("MCI error code {error}")
            } else {
                decode(&message)
            };
            return Err(HostError::new(format!(
                "Windows MCI command failed ({operation}, code {error}): {details}"
            )));
        }
        Ok(decode(&buffer))
    })
}

/// Sends a playback command on the MCI device's owning thread.
pub(super) fn mciCommand(command: &str) -> HostResult<()> {
    execute(command, false).map(|_| ())
}

/// Queries a playback device on the same thread that opened its alias.
pub(super) fn mciCommandWithResponse(command: &str) -> HostResult<String> {
    execute(command, true)
}

fn decode(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tts::WindowsTtsPlaybackHost;
    use operit_host_api::TtsPlaybackHost;
    use std::{fs, path::PathBuf, thread};
    use uuid::Uuid;

    /// Owns a ten-second silent PCM file so playback tests emit no speech.
    struct SilentWave(PathBuf);

    impl SilentWave {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("operit-mci-test-{}.wav", Uuid::new_v4()));
            let data_length = 8000u32 * 2 * 10;
            let mut bytes = Vec::with_capacity(44 + data_length as usize);
            bytes.extend_from_slice(b"RIFF");
            bytes.extend_from_slice(&(36 + data_length).to_le_bytes());
            bytes.extend_from_slice(b"WAVEfmt ");
            bytes.extend_from_slice(&16u32.to_le_bytes());
            bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
            bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
            bytes.extend_from_slice(&8000u32.to_le_bytes());
            bytes.extend_from_slice(&16000u32.to_le_bytes());
            bytes.extend_from_slice(&2u16.to_le_bytes());
            bytes.extend_from_slice(&16u16.to_le_bytes());
            bytes.extend_from_slice(b"data");
            bytes.extend_from_slice(&data_length.to_le_bytes());
            bytes.resize(44 + data_length as usize, 0);
            fs::write(&path, bytes).unwrap();
            Self(path)
        }
    }

    impl Drop for SilentWave {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn mci_alias_survives_different_calling_threads() {
        let wav = SilentWave::new();
        let alias = format!("operit_mci_test_{}", Uuid::new_v4().simple());
        let open = format!("open \"{}\" alias {alias}", wav.0.display());
        thread::spawn(move || mciCommand(&open))
            .join()
            .unwrap()
            .unwrap();
        let result = mciCommandWithResponse(&format!("status {alias} length"));
        let close = mciCommand(&format!("close {alias}"));
        assert_eq!(result.unwrap().trim(), "10000");
        close.unwrap();
    }

    #[test]
    #[ignore = "requires a Windows audio output device"]
    fn tts_can_be_polled_paused_resumed_and_stopped_from_other_threads() {
        let wav = SilentWave::new();
        let host = WindowsTtsPlaybackHost::new();
        assert!(host.playAudio(wav.0.to_str().unwrap()).unwrap().active);
        let other = host.clone();
        let result = thread::spawn(move || -> HostResult<()> {
            assert!(other.speechState()?.active);
            assert!(other.pauseSpeech()?.paused);
            assert!(!other.resumeSpeech()?.paused);
            Ok(())
        })
        .join()
        .unwrap();
        let other = host.clone();
        let stopped = thread::spawn(move || other.stopSpeech()).join().unwrap();
        let _ = host.stopSpeech();
        result.unwrap();
        assert!(!stopped.unwrap().active);
        assert!(!host.speechState().unwrap().active);
        assert!(wav.0.is_file(), "Caller-owned audio must not be removed");
    }
}
