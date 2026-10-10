//! Optional capture/upload service. Uses the existing admitted PeerLink Push,
//! with one capture and bounded PCM items. No socket or carrier is created here.
use operit_edge_contract::audio::*;
use operit_host_api::{AudioCaptureHost, HostRuntimeTaskSchedulerHost};
use operit_link::{CoreLinkError, CorePushRequest, CoreValue};
use operit_node_runtime::NodeServices::NodeServices;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;

struct Capture {
    receiver: String,
    cancel: watch::Sender<bool>,
}
pub struct EdgeAudioService {
    host: Arc<dyn AudioCaptureHost>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    captures: Arc<Mutex<HashMap<String, Capture>>>,
}
impl EdgeAudioService {
    pub fn new(
        host: Arc<dyn AudioCaptureHost>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    ) -> Self {
        Self {
            host,
            scheduler,
            captures: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn listInputs(&self) -> Result<CoreValue, CoreLinkError> {
        let inputs = self.host.listInputs().map_err(|e| error(e.to_string()))?;
        if inputs.len() > 16 {
            return Err(error("Audio host returned too many inputs"));
        }
        let value = operit_link::toCoreValue(inputs).map_err(|e| error(e.to_string()))?;
        if operit_link::encodeLink(&value)
            .map_err(|e| error(e.to_string()))?
            .len()
            > 4096
        {
            return Err(error("Audio input capabilities exceed 4096 bytes"));
        }
        Ok(value)
    }
    pub async fn start(
        &self,
        args: CoreValue,
        origin: &str,
        services: NodeServices,
    ) -> Result<CoreValue, CoreLinkError> {
        let request: StartAudioInput =
            operit_link::fromCoreValue(args).map_err(|e| error(e.to_string()))?;
        request.format.validate().map_err(error)?;
        if request.receiverNodeId != origin
            || request.streamId.len() != 36
            || uuid::Uuid::parse_str(&request.streamId).is_err()
            || request.inputId.is_empty()
            || request.inputId.len() > 128
            || request.maxDurationMs == 0
            || request.maxDurationMs > AUDIO_MAX_DURATION_MS
        {
            return Err(error("Invalid audio start request"));
        }
        let client = services
            .peers()
            .spacePushClient()
            .ok_or_else(|| error("No admitted Space return route"))?;
        requireAudioCapacity(&request.format, client.nominalBytesPerSecond())?;
        let (cancel, mut cancelled) = watch::channel(false);
        {
            let mut captures = self.captures.lock().unwrap();
            if !captures.is_empty() {
                return Err(error("An audio capture is already active"));
            }
            captures.insert(
                request.streamId.clone(),
                Capture {
                    receiver: origin.into(),
                    cancel,
                },
            );
        }
        let id = request.streamId.clone();
        // Remove a reserved capture if open is cancelled, rejected, or scheduling fails.
        let mut reservation = Reservation {
            captures: self.captures.clone(),
            id: id.clone(),
            armed: true,
        };
        let open = AudioIngressOpen {
            streamId: id.clone(),
            format: request.format.clone(),
        };
        let mut push = client
            .openPushTo(
                origin,
                CorePushRequest::new(
                    format!("audio-{id}"),
                    CORE_AUDIO_INGRESS_TARGET,
                    CORE_AUDIO_INGRESS_METHOD,
                )
                .withArgs(operit_link::toCoreValue(open).map_err(|e| error(e.to_string()))?),
            )
            .await?;
        let opened = tokio::time::timeout(
            Duration::from_secs(10),
            self.host.openInput(&request.inputId, &request.format),
        )
        .await
        .unwrap_or_else(|_| Err(operit_host_api::HostError::new("Microphone open timed out")));
        let source = match opened {
            Ok(source) => source,
            Err(e) => {
                let _ = push.send(end(Some(e.to_string()))).await;
                let _ = push.close().await;
                return Err(error(e.to_string()));
            }
        };
        let captures = self.captures.clone();
        let format = request.format.clone();
        let duration = request.maxDurationMs;
        let taskId = id.clone();
        self.scheduler.scheduleHostRuntimeAsyncTask("edge-audio-upload", Box::new(move || Box::pin(async move {
            let _reservation = Reservation { captures, id: taskId, armed: true };
            let mut source = source;
            let timeout = tokio::time::sleep(Duration::from_millis(duration as u64));
            tokio::pin!(timeout);
            let terminal = loop {
                if *cancelled.borrow() { break Some("Audio capture was stopped".into()); }
                let block = tokio::select! {
                    _ = cancelled.changed() => break Some("Audio capture was stopped".into()),
                    _ = &mut timeout => break None,
                    block = source.read() => block,
                };
                match block {
                    Ok(Some(bytes)) => {
                        if let Err(e) = format.validateChunk(&bytes) { break Some(e); }
                        // Existing Push sends await ordered ACKs. Slow transport/consumer
                        // must fail explicitly; never queue unboundedly or drop PCM.
                        let sent = tokio::time::timeout(Duration::from_secs(12), push.send(CoreValue::Bytes(bytes))).await;
                        match sent {
                            Ok(Ok(())) => {},
                            Ok(Err(e)) => break Some(e.to_string()),
                            Err(_) => break Some("Audio transport stalled for twelve seconds".into()),
                        }
                    }
                    Ok(None) => break None,
                    Err(e) => break Some(e.to_string()),
                }
            };
            // Release microphone before waiting for network cleanup.
            drop(source);
            let _ = tokio::time::timeout(Duration::from_secs(10), push.send(end(terminal))).await;
            let _ = tokio::time::timeout(Duration::from_secs(10), push.close()).await;
        }))).map_err(|e| error(e.to_string()))?;
        reservation.armed = false;
        operit_link::toCoreValue(serde_json::json!({"streamId":id}))
            .map_err(|e| error(e.to_string()))
    }
    pub fn stop(&self, args: CoreValue, origin: &str) -> Result<CoreValue, CoreLinkError> {
        let request: StopAudioInput =
            operit_link::fromCoreValue(args).map_err(|e| error(e.to_string()))?;
        let captures = self.captures.lock().unwrap();
        if let Some(capture) = captures.get(&request.streamId) {
            if capture.receiver != origin {
                return Err(error("Only the requesting Core can stop this input"));
            }
            capture.cancel.send_replace(true);
        }
        Ok(CoreValue::Bool(true))
    }
}
impl Drop for EdgeAudioService {
    fn drop(&mut self) {
        for capture in self.captures.lock().unwrap().values() {
            capture.cancel.send_replace(true);
        }
    }
}
struct Reservation {
    captures: Arc<Mutex<HashMap<String, Capture>>>,
    id: String,
    armed: bool,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if self.armed {
            self.captures.lock().unwrap().remove(&self.id);
        }
    }
}
fn error(message: impl Into<String>) -> CoreLinkError {
    CoreLinkError::new("EDGE_AUDIO_ERROR", message)
}
fn end(error: Option<String>) -> CoreValue {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("end".into(), CoreValue::Bool(true));
    if let Some(message) = error {
        let message: String = message.chars().take(120).collect();
        fields.insert("error".into(), CoreValue::String(message));
    }
    CoreValue::Map(fields)
}

fn requireAudioCapacity(
    format: &operit_host_api::AudioInputFormat,
    capacity: Option<u32>,
) -> Result<(), CoreLinkError> {
    if let Some(capacity) = capacity {
        let bytesPerSecond = format.sampleRateHz * format.channels as u32 * 2;
        if bytesPerSecond as u64 * 4 > capacity as u64 * 3 {
            return Err(error("Serial capacity is insufficient for this PCM format; configure matching higher PeerLink baud rates on both Hosts"));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_capacity_rejects_legacy_uart_and_preserves_headroom() {
        let format = operit_host_api::AudioInputFormat::default();
        assert!(requireAudioCapacity(&format, Some(115200 / 10)).is_err());
        assert!(requireAudioCapacity(&format, Some(921600 / 10)).is_ok());
        assert!(requireAudioCapacity(&format, None).is_ok());
        let stereo = operit_host_api::AudioInputFormat {
            sampleRateHz: 48000,
            channels: 2,
            ..Default::default()
        };
        assert!(requireAudioCapacity(&stereo, Some(921600 / 10)).is_err());
    }
}
