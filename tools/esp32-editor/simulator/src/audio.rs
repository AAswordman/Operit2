//! Deterministic, paced PCM input for the editor. This is a simulated device,
//! installed through the same optional Host capability used by real microphones.
use async_trait::async_trait;
use operit_host_api::{
    AudioCaptureHost, AudioCaptureSession, AudioInputDevice, AudioInputFormat, HostError,
    HostResult,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
struct Settings {
    blocks: u32,
    fail: bool,
}
struct State {
    settings: Settings,
    active: bool,
    blocks: u32,
    bytes: u32,
    error: Option<String>,
}
#[derive(Clone)]
pub struct SimulatorMicrophone {
    state: Arc<Mutex<State>>,
}
impl SimulatorMicrophone {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                settings: Settings {
                    blocks: 50,
                    fail: false,
                },
                active: false,
                blocks: 0,
                bytes: 0,
                error: None,
            })),
        }
    }
    pub fn configure(&self, input: &serde_json::Value) -> Result<serde_json::Value, String> {
        let duration = input["durationMs"]
            .as_u64()
            .ok_or("Missing audio durationMs")?;
        let fail = input["fail"].as_bool().ok_or("Missing audio fail flag")?;
        if duration < 20 || duration > 10000 || duration % 20 != 0 {
            return Err("Audio duration must be 20..10000 ms, in 20 ms steps".into());
        }
        let mut state = self.state.lock().unwrap();
        if state.active {
            return Err("Stop the active capture before changing its input".into());
        }
        state.settings = Settings {
            blocks: (duration / 20) as u32,
            fail,
        };
        drop(state);
        Ok(self.snapshot())
    }
    pub fn snapshot(&self) -> serde_json::Value {
        let state = self.state.lock().unwrap();
        serde_json::json!({"inputId": "sim-pcm", "durationMs": state.settings.blocks * 20,
            "fail": state.settings.fail, "active": state.active, "blocks": state.blocks,
            "byteLength": state.bytes, "error": state.error,
            "format": AudioInputFormat::default()})
    }
}
struct Recording {
    state: Arc<Mutex<State>>,
    settings: Settings,
    offset: u32,
    started: Instant,
}
#[async_trait]
impl AudioCaptureHost for SimulatorMicrophone {
    fn listInputs(&self) -> HostResult<Vec<AudioInputDevice>> {
        Ok(vec![AudioInputDevice {
            inputId: "sim-pcm".into(),
            name: "Simulated PCM microphone (440 Hz)".into(),
            formats: vec![AudioInputFormat::default()],
        }])
    }
    async fn openInput(
        &self,
        input: &str,
        format: &AudioInputFormat,
    ) -> HostResult<Box<dyn AudioCaptureSession>> {
        if input != "sim-pcm" || format != &AudioInputFormat::default() {
            return Err(HostError::new(
                "Unsupported simulator audio input or format",
            ));
        }
        let mut state = self.state.lock().unwrap();
        if state.active {
            return Err(HostError::new("Simulator microphone already active"));
        }
        state.active = true;
        state.blocks = 0;
        state.bytes = 0;
        state.error = None;
        Ok(Box::new(Recording {
            state: self.state.clone(),
            settings: state.settings,
            offset: 0,
            started: Instant::now(),
        }))
    }
}
#[async_trait]
impl AudioCaptureSession for Recording {
    async fn read(&mut self) -> HostResult<Option<Vec<u8>>> {
        if self.offset == self.settings.blocks {
            if self.settings.fail {
                self.state.lock().unwrap().error = Some("Simulated microphone overrun".into());
                return Err(HostError::new("Simulated microphone overrun"));
            }
            return Ok(None);
        }
        // 320 samples = 20 ms at 16 kHz. Absolute deadlines prevent cumulative drift.
        let deadline = self.started + Duration::from_millis((self.offset as u64 + 1) * 20);
        tokio::time::sleep_until(deadline.into()).await;
        let mut bytes = Vec::with_capacity(640);
        for sample in self.offset * 320..(self.offset + 1) * 320 {
            let value =
                ((sample as f64 * 440.0 * std::f64::consts::TAU / 16000.0).sin() * 8000.0) as i16;
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        self.offset += 1;
        let mut state = self.state.lock().unwrap();
        state.blocks += 1;
        state.bytes += bytes.len() as u32;
        Ok(Some(bytes))
    }
}
impl Drop for Recording {
    fn drop(&mut self) {
        self.state.lock().unwrap().active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn paced_input_reports_eof_overrun_and_releases_on_drop() {
        let mic = SimulatorMicrophone::new();
        mic.configure(&serde_json::json!({"durationMs": 20, "fail": false}))
            .unwrap();
        let mut input = mic
            .openInput("sim-pcm", &AudioInputFormat::default())
            .await
            .unwrap();
        assert!(mic
            .configure(&serde_json::json!({"durationMs": 20, "fail": true}))
            .is_err());
        let bytes = input.read().await.unwrap().unwrap();
        assert_eq!(bytes.len(), 640);
        assert_eq!(&bytes[..4], &[0, 0, 95, 5]);
        assert!(input.read().await.unwrap().is_none());
        drop(input);
        assert_eq!(mic.snapshot()["active"], false);
        mic.configure(&serde_json::json!({"durationMs": 20, "fail": true}))
            .unwrap();
        let mut input = mic
            .openInput("sim-pcm", &AudioInputFormat::default())
            .await
            .unwrap();
        input.read().await.unwrap();
        assert!(input
            .read()
            .await
            .unwrap_err()
            .to_string()
            .contains("overrun"));
        drop(input);
        assert_eq!(mic.snapshot()["active"], false);
        assert!(mic
            .configure(&serde_json::json!({"durationMs": 21, "fail": false}))
            .is_err());
    }
}
