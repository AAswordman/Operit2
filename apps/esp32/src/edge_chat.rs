//! Volatile UI state; all chat execution and persistence belongs to Space.
#![allow(non_snake_case)]
use operit_edge_transport::EdgeSpaceRouteClient;
use operit_link::{
    CoreCallRequest, CoreEventKind, CoreValue, CoreWatchRequest, CORE_INTERNAL_ROUTE_OBJECT_ID,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

struct ChatSession {
    client: EdgeSpaceRouteClient,
    chatId: String,
    runtime: tokio::runtime::Handle,
    messages: Mutex<CoreValue>,
    error: Mutex<Option<String>>,
    sending: AtomicBool,
    sendResult: Mutex<Option<Result<(), String>>>,
    streams: Mutex<BTreeMap<String, StreamText>>,
    streamTasks: Mutex<BTreeMap<String, tokio::task::JoinHandle<()>>>,
}

#[derive(Default)]
struct StreamText {
    text: String,
    completed: bool,
    savepoints: BTreeMap<String, String>,
}

impl StreamText {
    fn apply(&mut self, event: &serde_json::Value) {
        if !event
            .get("parentBlockId")
            .unwrap_or(&serde_json::Value::Null)
            .is_null()
        {
            return;
        }
        match event.get("type").and_then(|v| v.as_str()) {
            Some("reset") => {
                self.text.clear();
                self.savepoints.clear();
            }
            Some("chunk") => self
                .text
                .push_str(event.get("value").and_then(|v| v.as_str()).unwrap_or("")),
            Some("savepoint") => {
                if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                    self.savepoints.insert(id.into(), self.text.clone());
                }
            }
            Some("rollback") => {
                if let Some(text) = event
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|id| self.savepoints.get(id))
                {
                    self.text = text.clone();
                }
            }
            _ => {}
        }
    }
}

fn openMessageStreams(session: &Arc<ChatSession>, messages: &CoreValue) {
    let value = serde_json::to_value(messages).unwrap_or_default();
    let active = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|message| {
            message
                .pointer("/contentStream/$coreStream/streamId")
                .and_then(|v| v.as_str())
        })
        .collect::<std::collections::BTreeSet<_>>();
    session.streamTasks.lock().unwrap().retain(|id, task| {
        if active.contains(id.as_str()) {
            true
        } else {
            task.abort();
            false
        }
    });
    session
        .streams
        .lock()
        .unwrap()
        .retain(|id, _| active.contains(id.as_str()));
    for message in value.as_array().into_iter().flatten() {
        let Some(descriptor) = message
            .get("contentStream")
            .and_then(|v| v.get("$coreStream"))
        else {
            continue;
        };
        let Ok(descriptor) =
            serde_json::from_value::<operit_link::CoreStreamDescriptor>(descriptor.clone())
        else {
            continue;
        };
        {
            let mut streams = session.streams.lock().unwrap();
            if streams.contains_key(&descriptor.streamId) {
                continue;
            }
            streams.insert(descriptor.streamId.clone(), StreamText::default());
        }
        let streamId = descriptor.streamId.clone();
        let owner = session.clone();
        let session = session.clone();
        let task = tokio::spawn(async move {
            let mut args = match descriptor.args {
                CoreValue::Map(args) => args,
                _ => BTreeMap::new(),
            };
            args.insert(
                "streamId".into(),
                CoreValue::String(descriptor.streamId.clone()),
            );
            args.insert(
                operit_link::CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT.into(),
                CoreValue::String("chatMessagesFlow".into()),
            );
            args.insert(
                operit_link::CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT.into(),
                CoreValue::String("watch".into()),
            );
            args.insert(
                operit_link::CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT.into(),
                operit_link::toCoreValue(serde_json::json!({"chatId": session.chatId})).unwrap(),
            );
            let result = session
                .client
                .watchRouted(CoreWatchRequest::new(
                    operit_link::nextCoreRouteRequestId("openCoreStream"),
                    descriptor.targetObjectId,
                    descriptor.propertyName,
                    CoreValue::Map(args),
                ))
                .await;
            match result {
                Ok(mut stream) => {
                    while let Some(event) = stream.recv().await {
                        if event.kind == CoreEventKind::Completed {
                            break;
                        }
                        if let Some(text) = session
                            .streams
                            .lock()
                            .unwrap()
                            .get_mut(&descriptor.streamId)
                        {
                            text.apply(&serde_json::to_value(event.value).unwrap_or_default());
                        }
                    }
                }
                Err(error) => *session.error.lock().unwrap() = Some(error.to_string()),
            }
            if let Some(text) = session
                .streams
                .lock()
                .unwrap()
                .get_mut(&descriptor.streamId)
            {
                text.completed = true;
            }
        });
        owner.streamTasks.lock().unwrap().insert(streamId, task);
    }
}

static SESSION: OnceLock<Mutex<Option<Arc<ChatSession>>>> = OnceLock::new();

/// Called on the authenticated Link runtime after Space provisions the chat.
pub fn install(client: EdgeSpaceRouteClient, chatId: String) {
    let session = Arc::new(ChatSession {
        client,
        chatId,
        runtime: tokio::runtime::Handle::current(),
        messages: Mutex::new(CoreValue::List(Vec::new())),
        error: Mutex::new(None),
        sending: AtomicBool::new(false),
        sendResult: Mutex::new(None),
        streams: Mutex::new(BTreeMap::new()),
        streamTasks: Mutex::new(BTreeMap::new()),
    });
    *SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(session.clone());
    tokio::spawn(async move {
        let args = operit_link::toCoreValue(serde_json::json!({"chatId": session.chatId})).unwrap();
        let result = session
            .client
            .watchRouted(CoreWatchRequest::new(
                "edge-ui-messages",
                CORE_INTERNAL_ROUTE_OBJECT_ID,
                "chatMessagesFlow",
                args,
            ))
            .await;
        match result {
            Ok(mut stream) => {
                while let Some(event) = stream.recv().await {
                    if event.kind == CoreEventKind::Completed {
                        break;
                    }
                    let mut messages = session.messages.lock().unwrap();
                    if event.kind == CoreEventKind::Delta {
                        match messages.applyIncrementalDelta(&event.value) {
                            Ok(value) => *messages = value,
                            Err(error) => {
                                *session.error.lock().unwrap() = Some(error);
                                break;
                            }
                        }
                    } else {
                        *messages = event.value;
                    }
                    openMessageStreams(&session, &messages);
                }
                *session.error.lock().unwrap() = Some("聊天连接已断开，请等待重新连接".into());
                for (_, task) in std::mem::take(&mut *session.streamTasks.lock().unwrap()) {
                    task.abort();
                }
            }
            Err(error) => *session.error.lock().unwrap() = Some(error.to_string()),
        }
    });
}

/// Returns the live Space route state, rather than whether the pairing
/// listener is enabled.
pub fn isConnected() -> bool {
    SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|session| session.as_ref().map(|session| session.client.isConnected()))
        .unwrap_or(false)
}

/// Drops the local chat route and display state. Pairing credentials are
/// cleared separately by the Edge pairing authority.
pub fn clear() {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|mut session| session.take());
    if let Some(session) = session {
        for (_, task) in std::mem::take(&mut *session.streamTasks.lock().unwrap()) {
            task.abort();
        }
    }
}

pub fn snapshot() -> serde_json::Value {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone();
    match session {
        Some(session) => serde_json::json!({
            "connected": session.client.isConnected(), "chatId": session.chatId,
            "sending": session.sending.load(Ordering::Acquire),
            "messages": displayMessages(&session.messages.lock().unwrap(), &session.streams.lock().unwrap()), "error": *session.error.lock().unwrap(),
        }),
        None => {
            serde_json::json!({"connected": false, "messages": [], "error": "请先在 Space 中配对此设备"})
        }
    }
}

/// Small bounded summary for the physical display; full messages remain in Space.
pub fn preview() -> String {
    let snapshot = snapshot();
    if let Some(error) = snapshot.get("error").and_then(|v| v.as_str()) {
        return error.chars().take(72).collect();
    }
    if snapshot.get("connected").and_then(|v| v.as_bool()) != Some(true) {
        return "尚未连接对话".into();
    }
    snapshot
        .get("messages")
        .and_then(|v| v.as_array())
        .and_then(|items| items.last())
        .and_then(|message| message.get("text").and_then(|v| v.as_str()))
        .map(|text| text.chars().take(20).collect())
        .unwrap_or_else(|| "已连接 Operit".into())
}

/// Returns a compact state label for the 320x240 task rail.
pub fn taskStatus() -> String {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone();
    let Some(session) = session else {
        return "离线".into();
    };
    if session.error.lock().unwrap().is_some() {
        return "错误".into();
    }
    if !session.client.isConnected() {
        return "离线".into();
    }
    if session.sending.load(Ordering::Acquire) {
        return "发送中".into();
    }
    if session
        .streams
        .lock()
        .unwrap()
        .values()
        .any(|stream| !stream.completed)
    {
        return "生成中".into();
    }
    "就绪".into()
}

/// Returns one plain text value for the small ESP32 display.
///
/// The device keeps the complete conversation in Core/Space, but its 320x240
/// screen deliberately shows only the latest non-empty text. It is a compact
/// message terminal rather than a second full AI chat client.
pub fn screenText() -> String {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone();
    let Some(session) = session else {
        return "未连接 Operit".into();
    };
    if let Some(error) = session.error.lock().unwrap().as_deref() {
        return compactText(error, 640);
    }
    let messages = displayMessages(
        &session.messages.lock().unwrap(),
        &session.streams.lock().unwrap(),
    );
    if let Some(text) = messages
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .filter_map(|message| message.get("text").and_then(|value| value.as_str()))
        .find(|text| !text.trim().is_empty())
    {
        return compactText(text, 640);
    }
    if session.client.isConnected() {
        "已连接，输入消息后发送".into()
    } else {
        "设备已离线".into()
    }
}

fn compactText(text: &str, max: usize) -> String {
    let flattened = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = flattened.chars().take(max).collect::<String>();
    if flattened.chars().count() > max {
        result.push('…');
    }
    result
}

fn displayMessages(value: &CoreValue, streams: &BTreeMap<String, StreamText>) -> serde_json::Value {
    let mut messages = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
    if let Some(messages) = messages.as_array_mut() {
        for message in messages {
            let mut text = message
                .get("parts")
                .and_then(|v| v.as_array())
                .map(|parts| {
                    parts
                        .iter()
                        .filter(|p| {
                            matches!(
                                p.get("kind").and_then(|v| v.as_str()),
                                Some("markdown" | "status")
                            )
                        })
                        .filter_map(|p| p.get("content").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            if let Some(stream) = message
                .pointer("/contentStream/$coreStream/streamId")
                .and_then(|v| v.as_str())
                .and_then(|id| streams.get(id))
            {
                text = stream.text.clone();
            }
            message["text"] = text.into();
        }
    }
    messages
}

/// Bridges a synchronous firmware HTTP callback to its existing Link runtime.
pub fn send(text: String) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("消息不能为空".into());
    }
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "设备尚未连接 Space".to_string())?;
    if !session.client.isConnected() {
        return Err("设备已离线".into());
    }
    if session.sending.swap(true, Ordering::AcqRel) {
        return Err("上一条消息仍在发送".into());
    }
    *session.error.lock().unwrap() = None;
    let runtime = session.runtime.clone();
    runtime.spawn(async move {
        let args = operit_link::toCoreValue(serde_json::json!({
            "promptFunctionType": "CHAT", "roleCardIdOverride": null,
            "chatIdOverride": session.chatId, "messageText": text,
            "proxySenderNameOverride": null, "chatProviderIdOverride": null,
            "chatModelIdOverride": null, "attachments": [], "replyToMessage": null,
            "turnOptions": {"persistTurn": true, "notifyReply": null, "hideUserMessage": false,
                "disableWarning": false, "chatInputSubmitRequestedHandled": false},
        }))
        .unwrap();
        let response = session
            .client
            .callRouted(CoreCallRequest::new(
                operit_link::nextCoreRouteRequestId("sendUserMessage"),
                CORE_INTERNAL_ROUTE_OBJECT_ID,
                "sendUserMessage",
                args,
            ))
            .await;
        let result = response
            .result
            .map(|_| ())
            .map_err(|error| error.to_string());
        if let Err(error) = &result {
            *session.error.lock().unwrap() = Some(error.clone());
        }
        *session.sendResult.lock().unwrap() = Some(result);
        session.sending.store(false, Ordering::Release);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replyStreamHandlesRollbackWithoutDuplicatingRendererChunks() {
        let mut text = StreamText::default();
        for event in [
            serde_json::json!({"type":"reset"}),
            serde_json::json!({"type":"chunk","value":"你好"}),
            serde_json::json!({"type":"savepoint","id":"a"}),
            serde_json::json!({"type":"chunk","value":"错误分支"}),
            serde_json::json!({"type":"rollback","id":"a"}),
            serde_json::json!({"type":"markdownBlockChunk","value":"重复渲染数据"}),
            serde_json::json!({"type":"chunk","parentBlockId":1,"value":"嵌套块"}),
            serde_json::json!({"type":"chunk","value":"，世界"}),
        ] {
            text.apply(&event);
        }
        assert_eq!(text.text, "你好，世界");
        let messages = operit_link::toCoreValue(serde_json::json!([{
            "sender":"ai", "parts":[], "contentStream":{"$coreStream":{"streamId":"s"}}
        }]))
        .unwrap();
        let displayed = displayMessages(&messages, &BTreeMap::from([("s".into(), text)]));
        assert_eq!(displayed[0]["text"], "你好，世界");
    }
}

/// Acknowledge the remote send, not merely admission to the local queue.
pub fn takeSendResult() -> Option<Result<(), String>> {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()?;
    let result = session.sendResult.lock().unwrap().take();
    result
}
