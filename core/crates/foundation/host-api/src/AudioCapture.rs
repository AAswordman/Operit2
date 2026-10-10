//! Optional microphone capability. Samples are raw interleaved PCM, never a WAV file.
use crate::HostResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const AUDIO_MAX_CHUNK_BYTES: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioInputFormat {
    pub encoding: String,
    pub sampleRateHz: u32,
    pub channels: u8,
}
impl Default for AudioInputFormat {
    fn default() -> Self {
        Self {
            encoding: "pcm_s16le".into(),
            sampleRateHz: 16000,
            channels: 1,
        }
    }
}
impl AudioInputFormat {
    pub fn validate(&self) -> Result<(), String> {
        if self.encoding != "pcm_s16le"
            || !matches!(self.sampleRateHz, 8000 | 16000 | 24000 | 48000)
            || !matches!(self.channels, 1 | 2)
        {
            return Err("Unsupported PCM format".into());
        }
        Ok(())
    }
    pub fn validateChunk(&self, bytes: &[u8]) -> Result<(), String> {
        self.validate()?;
        if bytes.is_empty()
            || bytes.len() > AUDIO_MAX_CHUNK_BYTES
            || bytes.len() % (2 * self.channels as usize) != 0
        {
            return Err("PCM chunk must contain complete samples and 1..4096 bytes".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioInputDevice {
    pub inputId: String,
    pub name: String,
    pub formats: Vec<AudioInputFormat>,
}
/// Dropping a capture must stop recording and release the device. Implementations
/// must report capture overruns as errors, rather than silently losing samples.
#[async_trait]
pub trait AudioCaptureSession: Send {
    /// Returns a bounded PCM block, None for clean EOF, or an error for lost audio.
    async fn read(&mut self) -> HostResult<Option<Vec<u8>>>;
}
#[async_trait]
pub trait AudioCaptureHost: Send + Sync {
    fn listInputs(&self) -> HostResult<Vec<AudioInputDevice>>;
    /// Reject unsupported formats; do not silently resample or substitute devices.
    async fn openInput(
        &self,
        inputId: &str,
        format: &AudioInputFormat,
    ) -> HostResult<Box<dyn AudioCaptureSession>>;
}
