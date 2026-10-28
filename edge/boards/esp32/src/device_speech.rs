//! Optional speech offload protocol owned by the device application, not Edge runtime.
//! A capable device can use its own recognizer/synthesizer instead of opening this endpoint.
#![allow(non_snake_case)]
use operit_link::{
    CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError, CoreLinkPushSession, CorePushRequest,
    CoreValue, CoreWatchRequest,
};
use operit_edge_sdk::device::{EdgePushHandler, EdgeWatchHandler};
use std::{collections::BTreeMap, sync::{Arc, Mutex}};
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
    pendingText: String,
    error: Option<String>,
}
/// Bounded, device-owned session. No Core speech routes, leases, or plugin runtime.
#[derive(Clone)]
pub struct DeviceSpeech {
    state: Arc<Mutex<State>>,
    capture: Arc<tokio::sync::Mutex<()>>,
    changed: tokio::sync::watch::Sender<bool>,
    playback: mpsc::Sender<CoreValue>,
    playbackInput: Arc<Mutex<mpsc::Receiver<CoreValue>>>,
}
impl Default for DeviceSpeech {
    fn default() -> Self {
        let (playback, input) = mpsc::channel(4);
        Self {
            state: Arc::new(Mutex::new(State::default())),
            capture: Arc::new(tokio::sync::Mutex::new(())),
            changed: tokio::sync::watch::channel(false).0,
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
#[cfg(test)]
fn value(value: serde_json::Value) -> CoreValue {
    operit_link::toCoreValue(value).unwrap()
}
// Share one small wire builder across capture, cancellation and TTS. Avoid
// constructing a JSON tree and copying it back into Link values on the device.
#[inline(never)]
fn packetFields(kind: &str) -> BTreeMap<String, CoreValue> {
    let mut fields = BTreeMap::new();
    fields.insert("type".into(), CoreValue::String(kind.into()));
    fields
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
        let mut ready = packetFields("ready");
        ready.insert("protocol".into(), CoreValue::Unsigned(2));
        sender
            .try_send(CoreEvent {
                requestId: Some(request.requestId),
                target: TARGET.into(),
                propertyName: "requests".into(),
                kind: CoreEventKind::Snapshot,
                value: CoreValue::Map(ready),
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
        state.pendingText = String::new();
        self.changed.send_replace(true);
        if state.active.take().is_some() {
            while self.playbackInput.lock().unwrap().try_recv().is_ok() {}
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
    fn begin(&self, operation: &str, mut payload: BTreeMap<String, CoreValue>) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();
        if !state.inputOpen {
            return Err("请先连接语音处理插件".into());
        }
        if state.active.is_some() {
            return Err("请先结束当前语音任务".into());
        }
        if operation == "tts" {
            while self.playbackInput.lock().unwrap().try_recv().is_ok() {}
        }
        let id = uuid::Uuid::new_v4().to_string();
        payload.insert("id".into(), CoreValue::String(id.clone()));
        Self::emit(&mut state, CoreValue::Map(payload))?;
        state.active = Some((id.clone(), operation.into()));
        state.text = None;
        state.pendingText = String::new();
        state.error = None;
        state.recording = operation == "stt.begin";
        Ok(id)
    }
    /// Native capture supplies 16 kHz, mono, signed 16-bit PCM in bounded blocks.
    pub fn beginRecognition(&self) -> Result<String, String> {
        let mut payload = packetFields("stt.begin");
        for (key, number) in [("sampleRate", 16000), ("channels", 1), ("bitsPerSample", 16)] {
            payload.insert(key.into(), CoreValue::Unsigned(number));
        }
        self.begin("stt.begin", payload)
    }
    // Capture callers await this bounded queue. Never hold the state lock while
    // waiting: disconnect/cancel must invalidate an outstanding reservation.
    async fn capturePacket(&self, id: String, pcm: Option<Vec<u8>>) -> Result<(), String> {
        let mut changed = self.changed.subscribe();
        let sender = {
            let state = self.state.lock().unwrap();
            if !state.recording { return Err("Microphone is not recording".into()); }
            if state.active.as_ref().map(|p| &p.0) != Some(&id) { return Err("Speech request expired".into()); }
            state.processor.as_ref().ok_or("Speech processor disconnected")?.1.clone()
        };
        let permit = tokio::select! {
            result = sender.reserve() => result.map_err(|_| "Speech processor disconnected")?,
            _ = changed.changed() => return Err("Speech request expired".into()),
        };
        let mut state = self.state.lock().unwrap();
        if !state.recording || state.active.as_ref().map(|p| p.0.as_str()) != Some(id.as_str()) {
            return Err("Speech request expired".into());
        }
        let end = pcm.is_none();
        let mut packet = packetFields(if end {"stt.end"} else {"stt.data"});
        packet.insert("id".into(), CoreValue::String(id));
        if let Some(pcm) = pcm { packet.insert("audio".into(), CoreValue::Bytes(pcm.into())); }
        permit.send(CoreEvent { requestId: None, target: TARGET.into(), propertyName: "requests".into(),
            kind: CoreEventKind::Changed, value: CoreValue::Map(packet) });
        if end { state.recording = false; }
        Ok(())
    }
    pub fn recordingId(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        if state.recording { state.active.as_ref().map(|active| active.0.clone()) } else { None }
    }
    pub fn isRecording(&self, id: &str) -> bool {
        let state = self.state.lock().unwrap();
        state.recording && state.active.as_ref().is_some_and(|active| active.0 == id)
    }
    pub async fn appendPcm(&self, pcm: Vec<u8>) -> Result<(), String> {
        let id = self.recordingId().ok_or("Speech session ended")?;
        self.appendPcmFor(&id, pcm).await
    }
    /// A native capture task keeps its original identity across cancel/restart.
    pub async fn appendPcmFor(&self, id: &str, pcm: Vec<u8>) -> Result<(), String> {
        if pcm.is_empty() || pcm.len() > BLOCK || pcm.len() % 2 != 0 {
            return Err("Invalid PCM block".into());
        }
        let _capture = self.capture.lock().await;
        self.capturePacket(id.to_owned(), Some(pcm)).await
    }
    pub async fn finishRecognition(&self) -> Result<(), String> {
        let id = self.recordingId().ok_or("Speech session ended")?;
        self.finishRecognitionFor(&id).await
    }
    pub async fn finishRecognitionFor(&self, id: &str) -> Result<(), String> {
        let _capture = self.capture.lock().await;
        self.capturePacket(id.to_owned(), None).await
    }
    pub fn synthesize(&self, text: String) -> Result<String, String> {
        if text.trim().is_empty() || text.len() > 4096 {
            return Err("Invalid speech text".into());
        }
        let mut payload = packetFields("tts");
        payload.insert("text".into(), CoreValue::String(text));
        self.begin("tts", payload)
    }
    pub fn cancel(&self, message: String) {
        let mut state = self.state.lock().unwrap();
        self.changed.send_replace(true);
        if let Some((id, _)) = state.active.take() {
            let mut payload = packetFields("cancel");
            payload.insert("id".into(), CoreValue::String(id));
            let _ = Self::emit(&mut state, CoreValue::Map(payload));
            state.recording = false;
            state.pendingText = String::new();
            state.error = Some(message);
            state.resultId += 1;
        }
        while self.playbackInput.lock().unwrap().try_recv().is_ok() {}
    }
    /// The physical UI polls scalars and moves completed text once. Never clone a
    /// potentially 16 KiB recognition result into JSON on every display tick.
    pub fn takeNativeUpdate(&self, previous: &mut u64) -> (u32, Option<Result<String, String>>) {
        let mut state = self.state.lock().unwrap();
        let phase = if state.recording { 1 } else if state.active.is_some() { 3 } else { 0 };
        let result = if state.resultId != *previous {
            *previous = state.resultId;
            state.text.take().map(Ok).or_else(|| state.error.clone().map(Err))
        } else { None };
        (phase, result)
    }
    pub fn canPlay(&self, id: &str) -> bool {
        let state = self.state.lock().unwrap();
        state.error.is_none() && state.active.as_ref().is_none_or(|active| active.0 == id)
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
                "stt.text" if operation == "stt.begin" && !state.recording => {
                    let text = string(&packet, "text")?;
                    if text.is_empty() || text.len() > 2048 || state.pendingText.len() + text.len() > 16384 {
                        return Err(error("Invalid recognized text chunk"));
                    }
                    // Vec's geometric growth can otherwise reserve almost 32 KiB
                    // for a 16 KiB result after several non-power-of-two chunks.
                    state.pendingText.try_reserve_exact(text.len())
                        .map_err(|_| error("Recognized text allocation failed"))?;
                    state.pendingText.push_str(text);
                    return Ok(());
                }
                "stt.result" if operation == "stt.begin" && !state.recording => {
                    let text = string(&packet, "text")?;
                    if state.pendingText.len() + text.len() > 16384 {
                        return Err(error("Recognized text exceeds device capacity"));
                    }
                    // Vec's geometric growth can otherwise reserve almost 32 KiB
                    // for a 16 KiB result after several non-power-of-two chunks.
                    state.pendingText.try_reserve_exact(text.len())
                        .map_err(|_| error("Recognized text allocation failed"))?;
                    state.pendingText.push_str(text);
                    state.text = Some(std::mem::take(&mut state.pendingText));
                    state.active = None;
                    state.recording = false;
                    self.changed.send_replace(true);
                    state.resultId += 1;
                    return Ok(());
                }
                "error" => {
                    state.pendingText = String::new();
                    state.error = Some(string(&packet, "error")?.chars().take(256).collect());
                    state.active = None;
                    state.recording = false;
                    self.changed.send_replace(true);
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
        // Backpressure can yield while the user cancels or replaces a processor.
        // Reserve space first, then revalidate while holding the same lock as
        // cancel/disconnect so old audio cannot enter the next request's queue.
        let permit = self.playback.reserve().await.map_err(|_| error("Playback closed"))?;
        let mut state = self.state.lock().unwrap();
        if state.processor.as_ref().map(|p| p.0.as_str()) != Some(session)
            || state.active.as_ref().map(|p| p.0.as_str()) != Some(id.as_str()) {
            return Err(error("Speech request expired"));
        }
        permit.send(CoreValue::Map(packet));
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
    use std::future::Future;
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
        service.appendPcm(pcm.clone()).await.unwrap();
        assert_eq!(
            fields(packet(&mut output).await).unwrap()["audio"],
            CoreValue::Bytes(pcm.into())
        );
        service.finishRecognition().await.unwrap();
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
    async fn native_ui_moves_the_complete_recognition_result_only_once() {
        let service = DeviceSpeech::default();
        let (mut output, mut input) = connect(&service, "native-ui");
        packet(&mut output).await;
        let id = service.beginRecognition().unwrap(); packet(&mut output).await;
        service.finishRecognitionFor(&id).await.unwrap(); packet(&mut output).await;
        for _ in 0..8 {
            input.send(value(serde_json::json!({"id":id,"type":"stt.text","text":"x".repeat(2048)}))).await.unwrap();
        }
        input.send(value(serde_json::json!({"id":id,"type":"stt.result","text":""}))).await.unwrap();
        let mut previous = 0;
        let (phase, result) = service.takeNativeUpdate(&mut previous);
        assert_eq!(phase, 0); assert_eq!(result.unwrap().unwrap(), "x".repeat(16384));
        assert!(service.takeNativeUpdate(&mut previous).1.is_none());
        assert_eq!(service.snapshot()["text"], serde_json::Value::Null);
    }
    #[tokio::test]
    async fn previous_native_capture_cannot_append_or_finish_a_replacement_recording() {
        let service = DeviceSpeech::default();
        let (mut output, _input) = connect(&service, "native-restart");
        packet(&mut output).await;
        let old = service.beginRecognition().unwrap(); packet(&mut output).await;
        service.cancel("cancel".into()); packet(&mut output).await;
        let next = service.beginRecognition().unwrap(); packet(&mut output).await;
        assert!(!service.isRecording(&old)); assert!(service.isRecording(&next));
        assert!(service.appendPcmFor(&old, vec![0, 1]).await.is_err());
        assert!(service.finishRecognitionFor(&old).await.is_err());
        assert!(service.isRecording(&next));
        service.appendPcmFor(&next, vec![2, 3]).await.unwrap();
        assert_eq!(fields(packet(&mut output).await).unwrap()["id"], CoreValue::String(next));
    }
    #[tokio::test]
    async fn long_recognition_is_published_only_after_all_text_chunks_complete() {
        let service = DeviceSpeech::default();
        let (mut output, mut input) = connect(&service, "long-text");
        packet(&mut output).await;
        let id = service.beginRecognition().unwrap(); packet(&mut output).await;
        service.finishRecognition().await.unwrap(); packet(&mut output).await;
        let chunk = "字".repeat(682);
        for _ in 0..8 {
            input.send(value(serde_json::json!({"type":"stt.text","id":id,"text":chunk}))).await.unwrap();
            assert!(service.snapshot()["text"].is_null(), "partial input became a submitted draft");
        }
        input.send(value(serde_json::json!({"type":"stt.result","id":id,"text":"0123456789abcdef"}))).await.unwrap();
        assert_eq!(service.snapshot()["text"], chunk.repeat(8) + "0123456789abcdef");
        assert!(service.state.lock().unwrap().text.as_ref().unwrap().capacity() <= 16384);
        assert!(input.send(value(serde_json::json!({"type":"stt.text","id":id,"text":"stale"}))).await.is_err());
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
    async fn capture_waits_for_space_and_disconnect_releases_waiter() {
        let service = DeviceSpeech::default();
        let (mut output, input) = connect(&service, "capture");
        packet(&mut output).await;
        service.beginRecognition().unwrap();
        for _ in 0..3 { service.appendPcm(vec![1, 2]).await.unwrap(); }
        let append = service.appendPcm(vec![3, 4]);
        tokio::pin!(append);
        assert!(std::future::poll_fn(|cx| std::task::Poll::Ready(append.as_mut().poll(cx).is_pending())).await);
        assert_eq!(service.snapshot()["ready"], true);
        packet(&mut output).await;
        append.await.unwrap();
        let finish = service.finishRecognition();
        tokio::pin!(finish);
        assert!(std::future::poll_fn(|cx| std::task::Poll::Ready(finish.as_mut().poll(cx).is_pending())).await);
        drop(input);
        assert!(finish.await.is_err());
        assert_eq!(service.snapshot()["phase"], 0);
        drop(output);
        let (_output, _input) = connect(&service, "reconnected");
        assert!(service.beginRecognition().is_ok());
    }
    #[tokio::test]
    async fn five_minutes_of_pcm_is_ordered_with_a_slow_bounded_consumer() {
        let service = DeviceSpeech::default();
        let (mut output, _input) = connect(&service, "long-capture");
        packet(&mut output).await;
        service.beginRecognition().unwrap();
        const TOTAL: usize = 16000 * 2 * 300;
        let producer = async {
            for offset in (0..TOTAL).step_by(BLOCK) {
                let pcm = (offset..(offset + BLOCK).min(TOTAL)).map(|i| (i % 251) as u8).collect();
                service.appendPcm(pcm).await.unwrap();
            }
            service.finishRecognition().await.unwrap();
        };
        let consumer = async {
            let mut received = 0;
            loop {
                tokio::task::yield_now().await;
                let p = fields(packet(&mut output).await).unwrap();
                if p["type"] == CoreValue::String("stt.end".into()) { break; }
                if let Some(CoreValue::Bytes(bytes)) = p.get("audio") {
                    assert!(bytes.len() <= BLOCK);
                    for byte in bytes.iter() { assert_eq!(*byte, (received % 251) as u8); received += 1; }
                }
            }
            assert_eq!(received, TOTAL);
        };
        tokio::join!(producer, consumer);
        assert_eq!(service.snapshot()["ready"], true);
        assert_eq!(service.snapshot()["phase"], 3);
    }
    #[tokio::test]
    async fn cancelled_backpressured_audio_never_enters_the_next_request() {
        let service = DeviceSpeech::default();
        let (mut output, mut input) = connect(&service, "cancel-race");
        packet(&mut output).await;
        let id = service.synthesize("first".into()).unwrap();
        packet(&mut output).await;
        for _ in 0..4 {
            input.send(value(serde_json::json!({"type":"tts.segment","id":id,"contentType":"audio/wav"}))).await.unwrap();
        }
        let mut blocked = Box::pin(input.send(value(serde_json::json!({"type":"tts.segment","id":id}))));
        std::future::poll_fn(|cx| {
            assert!(std::future::Future::poll(blocked.as_mut(), cx).is_pending());
            std::task::Poll::Ready(())
        }).await;
        service.cancel("cancelled".into());
        let next = service.synthesize("second".into()).unwrap();
        assert!(blocked.await.is_err());
        assert!(service.takePlayback().is_none(), "cancelled audio crossed request boundary");
        assert_eq!(service.snapshot()["requestId"], next);
    }
    #[tokio::test]
    async fn rejected_synthesis_preserves_current_audio_and_disconnect_clears_it() {
        let service = DeviceSpeech::default();
        let (mut output, mut input) = connect(&service, "queue");
        packet(&mut output).await;
        let id = service.synthesize("first".into()).unwrap();
        input.send(value(serde_json::json!({"type":"tts.segment","id":id}))).await.unwrap();
        assert!(service.synthesize("busy".into()).is_err());
        assert!(service.takePlayback().is_some(), "rejected request discarded current playback");
        input.send(value(serde_json::json!({"type":"tts.segment","id":id}))).await.unwrap();
        drop(input);
        assert!(service.takePlayback().is_none(), "disconnected processor retained old playback");
        assert_eq!(service.snapshot()["phase"], 0);
    }

    #[tokio::test]
    async fn completed_audio_remains_playable_after_processor_closes() {
        let service = DeviceSpeech::default();
        let (mut output, mut input) = connect(&service, "completed");
        packet(&mut output).await;
        let id = service.synthesize("last segment".into()).unwrap();
        input.send(value(serde_json::json!({"type":"tts.segment","id":id}))).await.unwrap();
        input.send(value(serde_json::json!({"type":"tts.end","id":id}))).await.unwrap();
        drop(input);
        assert_eq!(fields(service.takePlayback().unwrap()).unwrap()["type"], CoreValue::String("tts.segment".into()));
        assert_eq!(fields(service.takePlayback().unwrap()).unwrap()["type"], CoreValue::String("tts.end".into()));
        assert!(service.takePlayback().is_none());
    }

}
