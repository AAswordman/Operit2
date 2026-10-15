//! Optional speech offload protocol owned by the device application, not Edge runtime.
//! A capable device can use its own recognizer/synthesizer instead of opening this endpoint.
#![allow(non_snake_case)]
use operit_link::{
    CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError, CoreLinkPushSession, CorePushRequest,
    CoreValue, CoreWatchRequest,
};
use operit_node_edge::{EdgePushHandler, EdgeWatchHandler};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
const TARGET: &str = "operit.device.speech";
const BLOCK: usize = 4096;

#[derive(Default)]
struct State {
    processor: Option<(String, mpsc::Sender<CoreEvent>)>,
    inputOpen: bool,
    active: Option<(String, String)>,
    recording: bool,
    resultId: u64,
    text: Option<String>,
    error: Option<String>,
}
/// Bounded, device-owned session. No Core speech routes, leases, or plugin runtime.
#[derive(Clone)]
pub struct DeviceSpeech {
    state: Arc<Mutex<State>>,
    playback: mpsc::Sender<CoreValue>,
    playbackInput: Arc<Mutex<mpsc::Receiver<CoreValue>>>,
}
impl Default for DeviceSpeech {
    fn default() -> Self {
        let (playback, input) = mpsc::channel(4);
        Self {
            state: Arc::new(Mutex::new(State::default())),
            playback,
            playbackInput: Arc::new(Mutex::new(input)),
        }
    }
}
fn error(message: impl Into<String>) -> CoreLinkError {
    CoreLinkError::new("DEVICE_SPEECH_ERROR", message)
}
fn string<'a>(
    fields: &'a std::collections::BTreeMap<String, CoreValue>,
    key: &str,
) -> Result<&'a str, CoreLinkError> {
    match fields.get(key) {
        Some(CoreValue::String(s)) => Ok(s),
        _ => Err(error(format!("Missing {key}"))),
    }
}
fn fields(
    value: CoreValue,
) -> Result<std::collections::BTreeMap<String, CoreValue>, CoreLinkError> {
    match value {
        CoreValue::Map(fields) => Ok(fields),
        _ => Err(error("Expected object")),
    }
}
fn value(value: serde_json::Value) -> CoreValue {
    operit_link::toCoreValue(value).unwrap()
}
impl DeviceSpeech {
    pub fn handlers(&self) -> (Arc<EdgeWatchHandler>, Arc<EdgePushHandler>) {
        let output = self.clone();
        let input = self.clone();
        (
            Arc::new(move |request| {
                let service = output.clone();
                Box::pin(async move { service.watch(request) })
            }),
            Arc::new(move |request| {
                let service = input.clone();
                Box::pin(async move { service.push(request) })
            }),
        )
    }
    fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.target != TARGET || request.propertyName != "requests" {
            return Err(CoreLinkError::watchNotFound(&request.registryKey()));
        }
        let args = fields(request.args)?;
        let session = string(&args, "session")?.to_owned();
        if session.is_empty() || session.len() > 128 {
            return Err(error("Invalid processor session"));
        }
        let mut state = self.state.lock().unwrap();
        if state.processor.is_some() {
            return Err(error("A speech processor is already connected"));
        }
        let (sender, stream) = CoreEventStream::boundedChannel(4);
        sender
            .try_send(CoreEvent {
                requestId: Some(request.requestId),
                target: TARGET.into(),
                propertyName: "requests".into(),
                kind: CoreEventKind::Snapshot,
                value: value(serde_json::json!({"type":"ready","protocol":1})),
            })
            .unwrap();
        state.processor = Some((session.clone(), sender));
        let service = self.clone();
        Ok(stream.withOnClose(move || service.disconnect(&session)))
    }
    fn push(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        if request.target != TARGET || request.methodName != "results" {
            return Err(CoreLinkError::methodNotFound(&format!(
                "{}.{}",
                request.target, request.methodName
            )));
        }
        let args = fields(request.args)?;
        let session = string(&args, "session")?.to_owned();
        let mut state = self.state.lock().unwrap();
        if state.processor.as_ref().map(|p| p.0.as_str()) != Some(session.as_str())
            || state.inputOpen
        {
            return Err(error("Processor session mismatch or duplicate input"));
        }
        state.inputOpen = true;
        Ok(Box::new(Input {
            service: self.clone(),
            session,
        }))
    }
    fn disconnect(&self, session: &str) {
        let mut state = self.state.lock().unwrap();
        if state.processor.as_ref().map(|p| p.0.as_str()) != Some(session) {
            return;
        }
        state.processor = None;
        state.inputOpen = false;
        if state.active.take().is_some() {
            state.recording = false;
            state.error = Some("语音处理端已断开".into());
            state.resultId += 1;
        }
    }
    fn emit(state: &mut State, payload: CoreValue) -> Result<(), String> {
        let result = state
            .processor
            .as_ref()
            .ok_or("请先连接语音处理插件")?
            .1
            .try_send(CoreEvent {
                requestId: None,
                target: TARGET.into(),
                propertyName: "requests".into(),
                kind: CoreEventKind::Changed,
                value: payload,
            });
        if result.is_err() {
            state.processor = None;
            state.inputOpen = false;
            return Err("语音通道繁忙或已关闭；本次请求已停止".into());
        }
        Ok(())
    }
    fn begin(&self, operation: &str, payload: serde_json::Value) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();
        if !state.inputOpen {
            return Err("请先连接语音处理插件".into());
        }
        if state.active.is_some() {
            return Err("请先结束当前语音任务".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let mut payload = payload.as_object().unwrap().clone();
        payload.insert("id".into(), id.clone().into());
        payload.insert("type".into(), operation.into());
        Self::emit(&mut state, value(payload.into()))?;
        state.active = Some((id.clone(), operation.into()));
        state.text = None;
        state.error = None;
        state.recording = operation == "stt.begin";
        Ok(id)
    }
    /// Native capture supplies 16 kHz, mono, signed 16-bit PCM in bounded blocks.
    pub fn beginRecognition(&self) -> Result<String, String> {
        self.begin(
            "stt.begin",
            serde_json::json!({"sampleRate":16000,"channels":1,"bitsPerSample":16}),
        )
    }
    pub fn appendPcm(&self, pcm: Vec<u8>) -> Result<(), String> {
        if pcm.is_empty() || pcm.len() > BLOCK || pcm.len() % 2 != 0 {
            return Err("Invalid PCM block".into());
        }
        let mut state = self.state.lock().unwrap();
        if !state.recording {
            return Err("Microphone is not recording".into());
        }
        let id = state
            .active
            .as_ref()
            .ok_or("Speech session ended")?
            .0
            .clone();
        let mut packet = fields(value(serde_json::json!({"type":"stt.data","id":id}))).unwrap();
        packet.insert("audio".into(), CoreValue::Bytes(pcm.into()));
        if let Err(message) = Self::emit(&mut state, CoreValue::Map(packet)) {
            state.active = None;
            state.recording = false;
            state.error = Some(message.clone());
            state.resultId += 1;
            return Err(message);
        }
        Ok(())
    }
    pub fn finishRecognition(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if !state.recording {
            return Err("Microphone is not recording".into());
        }
        let id = state.active.as_ref().unwrap().0.clone();
        if let Err(message) = Self::emit(
            &mut state,
            value(serde_json::json!({"type":"stt.end","id":id})),
        ) {
            state.active = None;
            state.recording = false;
            state.error = Some(message.clone());
            state.resultId += 1;
            return Err(message);
        }
        state.recording = false;
        Ok(())
    }
    pub fn synthesize(&self, text: String) -> Result<String, String> {
        if text.trim().is_empty() || text.len() > 16384 {
            return Err("Invalid speech text".into());
        }
        while self.playbackInput.lock().unwrap().try_recv().is_ok() {}
        self.begin("tts", serde_json::json!({"text":text}))
    }
    pub fn cancel(&self, message: String) {
        let mut state = self.state.lock().unwrap();
        if let Some((id, _)) = state.active.take() {
            let _ = Self::emit(
                &mut state,
                value(serde_json::json!({"type":"cancel","id":id})),
            );
            state.recording = false;
            state.error = Some(message);
            state.resultId += 1;
        }
        while self.playbackInput.lock().unwrap().try_recv().is_ok() {}
    }
    pub fn snapshot(&self) -> serde_json::Value {
        let state = self.state.lock().unwrap();
        serde_json::json!({"ready":state.processor.is_some()&&state.inputOpen,
            "phase":if state.recording {1} else if state.active.is_some() {3} else {0},
            "requestId":state.active.as_ref().map(|p|&p.0),"resultId":state.resultId,"text":state.text,"error":state.error})
    }
    /// The device's playback driver consumes these blocks. Simulator forwards them to Web Audio.
    pub fn takePlayback(&self) -> Option<CoreValue> {
        self.playbackInput.lock().unwrap().try_recv().ok()
    }
    async fn receive(&self, session: &str, packet: CoreValue) -> Result<(), CoreLinkError> {
        let packet = fields(packet)?;
        let id = string(&packet, "id")?.to_owned();
        let kind = string(&packet, "type")?.to_owned();
        {
            let mut state = self.state.lock().unwrap();
            if state.processor.as_ref().map(|p| p.0.as_str()) != Some(session) {
                return Err(error("Processor session expired"));
            }
            if state.active.as_ref().map(|p| p.0.as_str()) != Some(id.as_str()) {
                return Err(error("Speech request expired"));
            }
            let operation = &state.active.as_ref().unwrap().1;
            match kind.as_str() {
                "stt.result" if operation == "stt.begin" => {
                    let text = string(&packet, "text")?;
                    if text.len() > 16384 {
                        return Err(error("Recognized text exceeds device capacity"));
                    }
                    state.text = Some(text.to_owned());
                    state.active = None;
                    state.recording = false;
                    state.resultId += 1;
                    return Ok(());
                }
                "error" => {
                    state.error = Some(string(&packet, "error")?.chars().take(256).collect());
                    state.active = None;
                    state.recording = false;
                    state.resultId += 1;
                    return Ok(());
                }
                "tts.data" | "tts.segment" | "tts.end" if operation == "tts" => {}
                _ => return Err(error("Unexpected speech response")),
            }
            if kind == "tts.data" {
                match packet.get("audio") {
                    Some(CoreValue::Bytes(b)) if !b.is_empty() && b.len() <= BLOCK => {}
                    _ => return Err(error("Invalid synthesized audio block")),
                }
            }
        }
        self.playback
            .send(CoreValue::Map(packet))
            .await
            .map_err(|_| error("Playback closed"))?;
        let mut state = self.state.lock().unwrap();
        if kind == "tts.end" && state.active.as_ref().map(|p| p.0.as_str()) == Some(id.as_str()) {
            state.active = None;
            state.resultId += 1;
        }
        Ok(())
    }
}
struct Input {
    service: DeviceSpeech,
    session: String,
}
impl Drop for Input {
    fn drop(&mut self) {
        self.service.disconnect(&self.session);
    }
}
#[async_trait::async_trait]
impl CoreLinkPushSession for Input {
    async fn send(&mut self, packet: CoreValue) -> Result<(), CoreLinkError> {
        self.service.receive(&self.session, packet).await
    }
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connect(
        service: &DeviceSpeech,
        session: &str,
    ) -> (CoreEventStream, Box<dyn CoreLinkPushSession>) {
        let args = value(serde_json::json!({"session":session}));
        (
            service
                .watch(CoreWatchRequest::new(
                    "watch",
                    TARGET,
                    "requests",
                    args.clone(),
                ))
                .unwrap(),
            service
                .push(CorePushRequest::new("push", TARGET, "results").withArgs(args))
                .unwrap(),
        )
    }
    async fn packet(stream: &mut CoreEventStream) -> CoreValue {
        stream.recv().await.unwrap().value
    }
    #[tokio::test]
    async fn recognition_returns_text_to_device_over_watch_push_and_rejects_stale_results() {
        let service = DeviceSpeech::default();
        let (mut output, mut input) = connect(&service, "one");
        packet(&mut output).await;
        let id = service.beginRecognition().unwrap();
        assert!(service.synthesize("busy".into()).is_err());
        assert_eq!(
            fields(packet(&mut output).await).unwrap()["type"],
            CoreValue::String("stt.begin".into())
        );
        let pcm = vec![0, 128, 255, 127];
        service.appendPcm(pcm.clone()).unwrap();
        assert_eq!(
            fields(packet(&mut output).await).unwrap()["audio"],
            CoreValue::Bytes(pcm.into())
        );
        service.finishRecognition().unwrap();
        packet(&mut output).await;
        input
            .send(value(
                serde_json::json!({"id":id,"type":"stt.result","text":"设备识别成功"}),
            ))
            .await
            .unwrap();
        assert_eq!(service.snapshot()["text"], "设备识别成功");
        assert_eq!(service.snapshot()["phase"], 0);
        assert!(input
            .send(value(
                serde_json::json!({"id":id,"type":"stt.result","text":"stale"})
            ))
            .await
            .is_err());
    }
    #[tokio::test]
    async fn synthesis_returns_exact_binary_and_completion_to_device_playback() {
        let service = DeviceSpeech::default();
        let (mut output, mut input) = connect(&service, "two");
        packet(&mut output).await;
        let id = service.synthesize("你好设备".into()).unwrap();
        let request = fields(packet(&mut output).await).unwrap();
        assert_eq!(request["text"], CoreValue::String("你好设备".into()));
        input
            .send(value(
                serde_json::json!({"type":"tts.segment","id":id,"contentType":"audio/wav"}),
            ))
            .await
            .unwrap();
        let mut data = fields(value(serde_json::json!({"type":"tts.data","id":id}))).unwrap();
        data.insert("audio".into(), CoreValue::Bytes(vec![0, 128, 255].into()));
        input.send(CoreValue::Map(data)).await.unwrap();
        input
            .send(value(serde_json::json!({"type":"tts.end","id":id})))
            .await
            .unwrap();
        assert_eq!(service.snapshot()["phase"], 0);
        service.takePlayback().unwrap();
        assert_eq!(
            fields(service.takePlayback().unwrap()).unwrap()["audio"],
            CoreValue::Bytes(vec![0, 128, 255].into())
        );
        assert_eq!(
            fields(service.takePlayback().unwrap()).unwrap()["type"],
            CoreValue::String("tts.end".into())
        );
    }
    #[tokio::test]
    async fn disconnect_and_overrun_fail_explicitly_and_allow_a_new_processor() {
        let service = DeviceSpeech::default();
        let (mut output, input) = connect(&service, "three");
        packet(&mut output).await;
        service.beginRecognition().unwrap();
        drop(input);
        assert_eq!(service.snapshot()["phase"], 0);
        assert!(service.snapshot()["error"]
            .as_str()
            .unwrap()
            .contains("断开"));
        drop(output);
        let (mut output, _input) = connect(&service, "four");
        packet(&mut output).await;
        service.beginRecognition().unwrap();
        for _ in 0..3 {
            service.appendPcm(vec![1, 2]).unwrap();
        }
        assert!(service.appendPcm(vec![1, 2]).is_err());
        assert_eq!(service.snapshot()["phase"], 0);
        assert!(service.appendPcm(vec![1, 2]).is_err());
    }
}
