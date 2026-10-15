//! Device-owned, bounded chat. No Core, SQL, ToolPkg or peer connection is needed.
#![allow(non_snake_case)]
use operit_host_api::{HttpHost, HttpRequestData};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const MAX_MESSAGES: usize = 8;
const MAX_TEXT: usize = 2048;
const MAX_CONTEXT: usize = 6144;
const MAX_EVENT: usize = 8192;
static CHAT: OnceLock<Mutex<Option<Arc<Chat>>>> = OnceLock::new();
static REVISION: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, PartialEq)]
struct Config {
    endpoint: String,
    model: String,
    apiKey: String,
    systemPrompt: String,
}
impl Config {
    fn parse(value: &Value) -> Result<Self, String> {
        fn field(value: &Value, key: &str, limit: usize, required: bool) -> Result<String, String> {
            let value = value.get(key).and_then(Value::as_str).unwrap_or("");
            if value.len() > limit || (required && value.trim().is_empty()) {
                return Err(format!("Invalid chat.{key}"));
            }
            Ok(value.into())
        }
        let config = Self {
            endpoint: field(value, "endpoint", 1024, true)?,
            model: field(value, "model", 128, true)?,
            apiKey: field(value, "apiKey", 512, false)?,
            systemPrompt: field(value, "systemPrompt", 1024, false)?,
        };
        if !(config.endpoint.starts_with("https://") || config.endpoint.starts_with("http://"))
            || config.apiKey.contains(['\r', '\n'])
        {
            return Err("Invalid direct chat endpoint or credential".into());
        }
        Ok(config)
    }
}
struct State {
    id: String,
    messages: Vec<Value>,
    sending: bool,
    error: Option<String>,
    result: Option<Result<(), String>>,
}
struct Chat {
    config: Config,
    http: Arc<dyn HttpHost>,
    state: Mutex<State>,
}
fn slot() -> &'static Mutex<Option<Arc<Chat>>> {
    CHAT.get_or_init(|| Mutex::new(None))
}
fn chat() -> Option<Arc<Chat>> {
    slot().lock().unwrap().clone()
}
fn changed() {
    REVISION.fetch_add(1, Ordering::Relaxed);
}
fn tail(text: &mut String, limit: usize) {
    if text.len() > limit {
        let mut at = text.len() - limit;
        while !text.is_char_boundary(at) {
            at += 1;
        }
        text.drain(..at);
    }
}
fn bound(messages: &mut Vec<Value>) {
    while messages.len() > MAX_MESSAGES
        || (messages.len() > 2
            && messages
                .iter()
                .map(|m| m["text"].as_str().unwrap_or("").len())
                .sum::<usize>()
                > MAX_CONTEXT)
    {
        messages.drain(..2.min(messages.len()));
    }
}
/// The application selects direct chat by providing a valid `chat` preference.
/// Losing peers never resets this local conversation.
pub fn configure(http: Option<Arc<dyn HttpHost>>, config: Option<&Value>) -> Result<(), String> {
    let Some(value) = config else {
        if slot().lock().unwrap().take().is_some() {
            changed();
        }
        return Ok(());
    };
    let config = Config::parse(value)?;
    let http = http.ok_or("This host does not provide HTTP")?;
    let mut current = slot().lock().unwrap();
    if current.as_ref().is_some_and(|old| old.config == config) {
        return Ok(());
    }
    // Finish an in-flight request with its original configuration.
    if current
        .as_ref()
        .is_some_and(|old| old.state.lock().unwrap().sending)
    {
        return Ok(());
    }
    *current = Some(Arc::new(Chat {
        config,
        http,
        state: Mutex::new(State {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            sending: false,
            error: None,
            result: None,
        }),
    }));
    changed();
    Ok(())
}
pub fn active() -> bool {
    chat().is_some()
}
pub fn revision() -> u32 {
    REVISION.load(Ordering::Relaxed)
}
pub fn snapshot() -> Value {
    let Some(chat) = chat() else {
        return Value::Null;
    };
    let state = chat.state.lock().unwrap();
    json!({"connected":true,"mode":"direct","chatId":state.id,"initializing":false,
        "messages":state.messages,"sending":state.sending,"generating":state.sending,"error":state.error,
        "hasOlder":false,"hasNewer":false,"conversations":[{"id":state.id,"title":"本机对话"}]})
}
pub fn status() -> String {
    let Some(chat) = chat() else {
        return "未配置".into();
    };
    let state = chat.state.lock().unwrap();
    if state.sending {
        "生成中"
    } else if state.error.is_some() {
        "错误"
    } else {
        "就绪"
    }
    .into()
}
pub fn takeResult() -> Option<Result<(), String>> {
    chat()?.state.lock().unwrap().result.take()
}
pub fn newChat() -> Result<(), String> {
    let chat = chat().ok_or("Direct chat is not configured")?;
    let mut state = chat.state.lock().unwrap();
    if state.sending {
        return Err("请等待当前回复完成".into());
    }
    state.id = uuid::Uuid::new_v4().to_string();
    state.messages.clear();
    state.error = None;
    state.result = None;
    changed();
    Ok(())
}

#[derive(Default)]
struct Response {
    pending: Vec<u8>,
    status: Option<i32>,
    done: bool,
    error: Option<String>,
}
impl Response {
    fn chunk(&mut self, bytes: &[u8], chat: &Chat) {
        if self.error.is_some() {
            return;
        }
        for byte in bytes {
            if *byte == b'\n' {
                let line = std::mem::take(&mut self.pending);
                let line = match std::str::from_utf8(&line) {
                    Ok(line) => line.trim_end_matches('\r'),
                    Err(_) => {
                        self.error = Some("Invalid UTF-8 event".into());
                        return;
                    }
                };
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    self.done = true;
                    continue;
                }
                let value: Value = match serde_json::from_str(data) {
                    Ok(value) => value,
                    Err(_) => {
                        self.error = Some("Invalid chat stream event".into());
                        return;
                    }
                };
                if value.get("error").is_some() {
                    self.error = Some("Chat service returned an error".into());
                    return;
                }
                if value["choices"][0]["finish_reason"].as_str().is_some() {
                    self.done = true;
                }
                if let Some(text) = value["choices"][0]["delta"]["content"].as_str() {
                    let mut state = chat.state.lock().unwrap();
                    if let Some(message) = state.messages.last_mut() {
                        let mut content = message["text"].as_str().unwrap_or("").to_owned();
                        content.push_str(text);
                        tail(&mut content, MAX_TEXT);
                        message["text"] = content.into();
                    }
                    changed();
                }
            } else {
                if self.pending.len() >= MAX_EVENT {
                    self.error = Some("Chat stream event exceeds the device budget".into());
                    return;
                }
                self.pending.push(*byte);
            }
        }
    }
}
pub fn send(text: String) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("消息不能为空".into());
    }
    if text.len() > MAX_TEXT {
        return Err("消息超出设备长度限制".into());
    }
    let chat = chat().ok_or("Direct chat is not configured")?;
    let messages = {
        let mut state = chat.state.lock().unwrap();
        if state.sending {
            return Err("上一条消息仍在发送".into());
        }
        state.sending = true;
        state.error = None;
        state.result = None;
        state.messages.push(json!({"sender":"user","text":text}));
        bound(&mut state.messages);
        let mut messages = Vec::new();
        if !chat.config.systemPrompt.is_empty() {
            messages.push(json!({"role":"system","content":chat.config.systemPrompt}));
        }
        for message in &state.messages {
            messages.push(json!({"role":if message["sender"] == "user" { "user" } else { "assistant" },"content":message["text"]}));
        }
        state.messages.push(json!({"sender":"ai","text":""}));
        bound(&mut state.messages);
        messages
    };
    changed();
    let streamId = format!("device-chat-{}", uuid::Uuid::new_v4());
    let mut headers = vec![
        ("Content-Type".into(), "application/json".into()),
        ("Accept".into(), "text/event-stream".into()),
    ];
    if !chat.config.apiKey.is_empty() {
        headers.push((
            "Authorization".into(),
            format!("Bearer {}", chat.config.apiKey),
        ));
    }
    let request = HttpRequestData {
        url: chat.config.endpoint.clone(),
        method: "POST".into(),
        headers,
        body: serde_json::to_vec(
            &json!({"model":chat.config.model,"messages":messages,"stream":true,"max_tokens":256}),
        )
        .unwrap(),
        formFields: vec![],
        fileParts: vec![],
        connectTimeoutSeconds: 10,
        readTimeoutSeconds: 45,
        followRedirects: false,
        ignoreSsl: false,
        proxyHost: String::new(),
        proxyPort: 0,
    };
    let response = Arc::new(Mutex::new(Response::default()));
    let head = response.clone();
    let chunk = response.clone();
    let close = response.clone();
    let output = chat.clone();
    let finish = chat.clone();
    let result = chat.http.openHttpResponseStream(
        streamId,
        request,
        Arc::new(move |headValue| {
            head.lock().unwrap().status = Some(headValue.statusCode);
        }),
        Arc::new(move |bytes| {
            chunk.lock().unwrap().chunk(&bytes, &output);
        }),
        Arc::new(move |closed| {
            let mut response = close.lock().unwrap();
            if !response.pending.is_empty() {
                response.chunk(b"\n", &finish);
            }
            let result = closed.and_then(|_| {
                if response
                    .status
                    .is_none_or(|status| !(200..300).contains(&status))
                {
                    return Err(format!("Chat HTTP {}", response.status.unwrap_or(0)));
                }
                if let Some(error) = &response.error {
                    return Err(error.clone());
                }
                if !response.done {
                    return Err("聊天连接提前中断".into());
                }
                Ok(())
            });
            let mut state = finish.state.lock().unwrap();
            if let Err(error) = &result {
                state.error = Some(error.clone());
            }
            state.sending = false;
            state.result = Some(result);
            changed();
        }),
    );
    if let Err(error) = result {
        let message = error.to_string();
        let mut state = chat.state.lock().unwrap();
        state.sending = false;
        state.error = Some(message.clone());
        state.result = Some(Err(message.clone()));
        changed();
        return Err(message);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::*;
    struct UnusedHttp;
    impl HttpHost for UnusedHttp {
        fn imageDelivery(&self) -> HttpImageDelivery {
            HttpImageDelivery::Bytes
        }
        fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
            unreachable!()
        }
        fn downloadFiles(
            &self,
            _: HttpDownloadRequest,
            _: HttpDownloadControl,
            _: HttpDownloadProgressCallback,
        ) -> HostResult<HttpDownloadResult> {
            unreachable!()
        }
    }
    impl HttpStreamHost for UnusedHttp {
        fn openHttpByteStream(
            &self,
            _: String,
            _: HttpRequestData,
            _: HttpStreamOpenedCallback,
            _: HttpStreamChunkCallback,
            _: HttpStreamClosedCallback,
        ) -> HostResult<()> {
            unreachable!()
        }
        fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
            unreachable!()
        }
    }
    fn fixture() -> Chat {
        Chat {
            config: Config::parse(
                &json!({"endpoint":"https://example.invalid/chat","model":"test"}),
            )
            .unwrap(),
            http: Arc::new(UnusedHttp),
            state: Mutex::new(State {
                id: "test".into(),
                messages: vec![json!({"sender":"ai","text":""})],
                sending: true,
                error: None,
                result: None,
            }),
        }
    }
    #[test]
    fn byte_fragmented_utf8_stream_updates_bounded_device_context() {
        let chat = fixture();
        let mut response = Response::default();
        let text = "你好，ESP。";
        let event = format!(
            "data: {}\r\n\r\n",
            json!({"choices":[{"delta":{"content":text}}]})
        );
        for _ in 0..500 {
            for byte in event.bytes() {
                response.chunk(&[byte], &chat);
            }
        }
        response.chunk(b"data: [DONE]\n\n", &chat);
        assert!(response.done);
        assert!(response.error.is_none());
        let state = chat.state.lock().unwrap();
        let output = state.messages[0]["text"].as_str().unwrap();
        assert!(output.len() <= MAX_TEXT);
        assert!(output.ends_with(text));
    }
    #[test]
    fn malformed_or_oversized_streams_fail_without_unbounded_buffers() {
        let chat = fixture();
        let mut response = Response::default();
        response.chunk(&vec![b'x'; MAX_EVENT + 1], &chat);
        assert!(response.error.is_some());
        assert_eq!(response.pending.len(), MAX_EVENT);
        let mut response = Response::default();
        response.chunk(b"data: not-json\n", &chat);
        assert!(response.error.is_some());
        assert!(!response.done);
        let mut response = Response::default();
        response.chunk(
            b"data: {\"error\":{\"message\":\"private details\"}}\n",
            &chat,
        );
        assert_eq!(
            response.error.as_deref(),
            Some("Chat service returned an error")
        );
    }
    #[test]
    fn context_trimming_keeps_complete_turns_within_device_budget() {
        let mut messages = Vec::new();
        for _ in 0..30 {
            messages.push(json!({"sender":"user","text":"u".repeat(1000)}));
            bound(&mut messages);
            messages.push(json!({"sender":"ai","text":"a".repeat(1000)}));
            bound(&mut messages);
        }
        assert_eq!(messages[0]["sender"], "user");
        assert_eq!(messages.last().unwrap()["sender"], "ai");
        assert!(messages.len() <= MAX_MESSAGES);
        assert!(
            messages
                .iter()
                .map(|m| m["text"].as_str().unwrap().len())
                .sum::<usize>()
                <= MAX_CONTEXT
        );
        assert!(Config::parse(&json!({"endpoint":"file:///tmp/no","model":"test"})).is_err());
        assert!(Config::parse(
            &json!({"endpoint":"https://example.invalid","model":"test","apiKey":"bad\r\nheader"})
        )
        .is_err());
    }
}
