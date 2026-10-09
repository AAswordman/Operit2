//! Volatile UI state; all chat execution and persistence belongs to Space.
#![allow(non_snake_case)]
#[path = "edge_events.rs"]
mod edge_events;
pub use edge_events::startEdgeEvents;

use operit_link::CoreLinkSharedClient;
use operit_link::{
    CoreCallRequest, CoreEventKind, CoreValue, CoreWatchRequest, CORE_INTERNAL_TARGET,
};
use operit_node_runtime::NodeServices::NodeServices;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub(crate) struct NodeUiTask(tokio::sync::watch::Sender<bool>);
impl NodeUiTask {
    pub fn abort(&self) {
        let _ = self.0.send(true);
    }
}
pub(crate) fn spawnNodeUiTask<F>(task: impl FnOnce() -> F + Send + 'static) -> NodeUiTask
where
    F: std::future::Future<Output = ()> + 'static,
{
    spawnNodeUiSubscription(move |mut cancelled| async move {
        tokio::select! {
            _ = uiTaskCancelled(&mut cancelled) => {},
            _ = task() => {}
        }
    })
}

// A watch-open is an in-flight Link transaction. Cancelling it poisons the
// shared carrier, including unrelated UI calls. Let it finish, then drop the
// returned stream normally (which sends watch-close). Cancel only idle recv.
fn spawnNodeUiSubscription<F>(
    task: impl FnOnce(tokio::sync::watch::Receiver<bool>) -> F + Send + 'static,
) -> NodeUiTask
where
    F: std::future::Future<Output = ()> + 'static,
{
    let (cancel, cancelled) = tokio::sync::watch::channel(false);
    operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask("edge-ui", Box::new(move || Box::pin(task(cancelled))))
        .expect("Edge UI task must be scheduled by Host");
    NodeUiTask(cancel)
}

async fn uiTaskCancelled(cancelled: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *cancelled.borrow() { return; }
        if cancelled.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

async fn openUiSubscription(
    cancelled: &tokio::sync::watch::Receiver<bool>,
    open: impl std::future::Future<Output = Result<operit_link::CoreEventStream, operit_link::CoreLinkError>>,
) -> Result<Option<operit_link::CoreEventStream>, operit_link::CoreLinkError> {
    if *cancelled.borrow() { return Ok(None); }
    let result = open.await;
    if *cancelled.borrow() {
        // Including late success: release the remote watch without cancelling
        // the acknowledgement transaction or retaining an obsolete UI session.
        drop(result);
        Ok(None)
    } else { result.map(Some) }
}

async fn nextUiEvent(
    stream: &mut operit_link::CoreEventStream,
    cancelled: &mut tokio::sync::watch::Receiver<bool>,
) -> Option<operit_link::CoreEvent> {
    tokio::select! {
        biased;
        _ = uiTaskCancelled(cancelled) => None,
        event = stream.recv() => event,
    }
}

// Lossless, volatile list cache. Each element retains the original Link value
// encoding. A nested delta decodes only its addressed element, never the whole
// transcript/history tree; the same generic delta engine handles nested paths.
#[derive(Default)]
struct UiValue {
    encoded: Vec<Vec<u8>>,
    display: serde_json::Value,
}
impl UiValue {
    fn replace(&mut self, value: CoreValue) -> Result<(), String> {
        let CoreValue::List(items) = value else { return Err("Expected a UI list snapshot".into()); };
        let mut encoded = Vec::with_capacity(items.len());
        for mut item in items {
            boundCoreValue(&mut item, 0);
            encoded.push(operit_link::encodeLink(item).map_err(|error| error.to_string())?);
        }
        self.encoded = encoded;
        Ok(())
    }
    #[cfg(not(target_os = "espidf"))]
    fn update(&mut self, event: operit_link::CoreEvent) -> Result<(), String> {
        if event.kind != CoreEventKind::Delta { return self.replace(event.value); }
        let CoreValue::Map(fields) = event.value else { return Err("Expected incremental delta map".into()); };
        let Some(CoreValue::List(operations)) = fields.get("$coreDelta") else { return Err("Missing incremental delta marker".into()); };
        // Only compact bytes are cloned for rollback, never the large map tree.
        let mut next = Self { encoded: self.encoded.clone(), display: serde_json::Value::Null };
        for operation in operations {
            let CoreValue::Map(fields) = operation else { return Err("Expected incremental operation map".into()); };
            let Some(CoreValue::String(op)) = fields.get("op") else { return Err("Missing incremental operation".into()); };
            let Some(CoreValue::List(path)) = fields.get("path") else { return Err("Missing incremental path".into()); };
            if path.is_empty() {
                if op != "set" { return Err("Cannot remove UI list root".into()); }
                next.replace(fields.get("value").cloned().ok_or("Missing incremental value")?)?;
                continue;
            }
            let CoreValue::Unsigned(index) = path[0] else { return Err("Expected unsigned UI list index".into()); };
            let index = usize::try_from(index).map_err(|_| "UI list index exceeds platform limit")?;
            if path.len() == 1 && op == "remove" {
                if index >= next.encoded.len() { return Err("Invalid UI list removal index".into()); }
                next.encoded.remove(index);
                continue;
            }
            if index > next.encoded.len() || (index == next.encoded.len() && (path.len() != 1 || op != "set")) {
                return Err("Invalid UI list delta index".into());
            }
            let base = if index < next.encoded.len() {
                operit_link::decodeLink::<CoreValue>(&next.encoded[index]).map_err(|error| error.to_string())?
            } else { CoreValue::Null };
            let mut operation = fields.clone();
            operation.insert("path".into(), CoreValue::List(path[1..].to_vec()));
            let delta = CoreValue::Map(BTreeMap::from([("$coreDelta".into(), CoreValue::List(vec![CoreValue::Map(operation)]))]));
            let mut item = base.intoIncrementalDelta(&delta)?;
            boundCoreValue(&mut item, 0);
            if index == next.encoded.len() { next.encoded.push(Vec::new()); }
            operit_link::encodeLinkInto(item, &mut next.encoded[index]).map_err(|error| error.to_string())?;
        }
        self.encoded = next.encoded;
        Ok(())
    }
    fn forEachLast(&self, count: usize, mut project: impl FnMut(CoreValue)) -> Result<(), String> {
        let start = self.encoded.len().saturating_sub(count);
        for bytes in &self.encoded[start..] {
            project(operit_link::decodeLink(bytes).map_err(|error| error.to_string())?);
        }
        Ok(())
    }
    #[cfg(test)]
    fn decode(&self) -> Result<CoreValue, String> {
        let mut items = Vec::new();
        self.forEachLast(self.encoded.len(), |item| items.push(item))?;
        Ok(CoreValue::List(items))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PageCursor { timestamp: i64, offset: u32 }

struct ChatSession {
    spaceIdentity: Option<operit_node_runtime::NodeServices::SpaceClientIdentity>,
    client: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    services: NodeServices,
    histories: Mutex<UiValue>,
    tasks: Mutex<Vec<NodeUiTask>>,
    chatId: String,
    ready: AtomicBool,
    retired: AtomicBool,
    needsReconnect: AtomicBool,
    messages: Mutex<UiValue>,
    window: tokio::sync::watch::Sender<Option<PageCursor>>,
    older: Mutex<Option<PageCursor>>,
    windowHistory: Mutex<Vec<Option<PageCursor>>>,
    error: Mutex<Option<String>>,
    executionError: Mutex<Option<String>>,
    generating: AtomicBool,
    sending: AtomicBool,
    sendResult: Mutex<Option<Result<(), String>>>,
    subscriptionsClosed: AtomicBool,
    streams: Mutex<BTreeMap<String, StreamText>>,
    streamTasks: Mutex<BTreeMap<String, NodeUiTask>>,
    plugins: Mutex<PluginUiState>,
}

impl ChatSession {
    fn addTask(&self, task: NodeUiTask) {
        let mut tasks = self.tasks.lock().unwrap();
        if self.retired.load(Ordering::Acquire) { task.abort(); }
        else { tasks.push(task); }
    }
}

/// Reconnects the volatile UI even when entry replacement happens between polls.
/// Retains only the selected ID in the same Space, never a transcript replica.
#[derive(Default)]
pub struct SpaceChatRoute {
    identity: Option<operit_node_runtime::NodeServices::SpaceClientIdentity>,
    chatId: String,
    provision: bool,
    installed: bool,
    lastAttempt: Option<std::time::Instant>,
}
impl SpaceChatRoute {
    pub fn reset(&mut self) { clear(); *self = Self::default(); }
    pub fn poll(&mut self, services: NodeServices) {
        let session = SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap().clone();
        if self.installed {
            if let Some(session) = &session {
                self.chatId = session.chatId.clone();
                self.provision = !session.ready.load(Ordering::Acquire);
            }
        }
        let Some(connection) = services.peers().spaceConnection() else {
            if self.installed { clear(); self.installed = false; }
            return;
        };
        let changed = self.identity.as_ref() != Some(&connection.identity);
        if changed { self.provision = true; }
        if self.identity.as_ref().is_none_or(|old| old.spaceId != connection.identity.spaceId) {
            self.chatId = uuid::Uuid::new_v4().to_string();
            self.provision = true;
        }
        let ended = session.as_ref().is_some_and(|s| s.needsReconnect.load(Ordering::Acquire)
            || s.subscriptionsClosed.load(Ordering::Acquire));
        let retryDue = self.lastAttempt.is_none_or(|at| at.elapsed() >= std::time::Duration::from_secs(3));
        if changed || !self.installed || (ended && retryDue) || session.is_none() {
            installSession(connection.client, services, self.chatId.clone(), self.provision);
            self.identity = Some(connection.identity);
            self.installed = true;
            self.lastAttempt = Some(std::time::Instant::now());
        }
    }
}

fn currentSession(session: &Arc<ChatSession>) -> bool {
    SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap().as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, session))
}

// Normal termination of a primary watch requires fresh snapshots/subscriptions.
// Intentional cancellation of an obsolete session must not restart it.
struct SubscriptionLifetime {
    session: Arc<ChatSession>,
    cancelled: tokio::sync::watch::Receiver<bool>,
}
impl Drop for SubscriptionLifetime {
    fn drop(&mut self) {
        if !*self.cancelled.borrow() {
            self.session.needsReconnect.store(true, Ordering::Release);
        }
    }
}

fn isMissingSpaceRoute(error: &operit_link::CoreLinkError) -> bool {
    let text = error.to_string();
    text.contains("SPACE_ROUTE_NOT_FOUND") || text.contains("route is not registered")
}

async fn watchChatMessages(
    client: &Arc<dyn CoreLinkSharedClient + Send + Sync>,
    chatId: &str,
    cursor: Option<PageCursor>,
) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {
    let args = windowArgs(chatId, cursor);
    match client
        .watch(CoreWatchRequest::new(
            "edge-ui-messages",
            CORE_INTERNAL_TARGET,
            "chatMessagesWindowFlow",
            args,
        ))
        .await
    {
        Ok(stream) => Ok(stream),
        Err(error) if isMissingSpaceRoute(&error) => Err(operit_link::CoreLinkError::new(
            "EDGE_CHAT_UNSUPPORTED",
            "当前 Core 不支持轻量聊天接口，请升级 Core 后重新连接",
        )),
        Err(error) => Err(error),
    }
}

fn windowArgs(chatId: &str, cursor: Option<PageCursor>) -> CoreValue {
    operit_link::toCoreValue(serde_json::json!({"chatId":chatId,
        "beforeTimestamp":cursor.map(|c| c.timestamp), "beforeTextOffset":cursor.map(|c| c.offset),
        "textBytes":MAX_CHAT_STRING_BYTES, "textLines":MAX_CHAT_LINES})).unwrap()
}

#[derive(Default, Clone)]
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
            Some("chunk") => appendTail(
                &mut self.text,
                event.get("value").and_then(|v| v.as_str()).unwrap_or(""),
                MAX_CHAT_STRING_BYTES,
            ),
            Some("toolStatus") => {
                // A read-only Core display event replaces the most recent pending
                // call with this name. No result body or call arguments are retained.
                let token = event.get("value").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(name) = toolTokenName(token) {
                    let running = format!("\x1eR|{name}\x1f");
                    if let Some(at) = self.text.rfind(&running) {
                        self.text.replace_range(at..at + running.len(), token);
                    }
                }
            }
            Some("savepoint") => {
                if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                    if self.savepoints.len() >= 4 {
                        self.savepoints.clear();
                    }
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
        if let Some(start) = self.text.match_indices('\n').rev().nth(MAX_CHAT_LINES - 1).map(|(at, _)| at + 1) {
            self.text.drain(..start);
        }
    }
}

const MAX_STREAM_DESCRIPTORS: usize = 8;
const MAX_STREAM_SCAN_DEPTH: usize = 8;
const MAX_STREAM_SCAN_ITEMS: usize = 32;

fn collectMessageStreams(value: &CoreValue, streams: &mut Vec<operit_link::CoreStreamDescriptor>) {
    collectMessageStreamsBounded(value, streams, 0);
}

fn collectMessageStreamsBounded(
    value: &CoreValue,
    streams: &mut Vec<operit_link::CoreStreamDescriptor>,
    depth: usize,
) {
    if depth > MAX_STREAM_SCAN_DEPTH || streams.len() >= MAX_STREAM_DESCRIPTORS {
        return;
    }
    match value {
        CoreValue::List(items) => {
            for item in items.iter().take(MAX_STREAM_SCAN_ITEMS) {
                collectMessageStreamsBounded(item, streams, depth + 1);
            }
        }
        CoreValue::Map(fields) => {
            if let Some(CoreValue::Map(content_stream)) = fields.get("contentStream") {
                if let Some(CoreValue::Map(descriptor)) = content_stream.get("$coreStream") {
                    let streamId = match descriptor.get("streamId") {
                        Some(CoreValue::String(value)) => value.clone(),
                        _ => String::new(),
                    };
                    let target = match descriptor.get("target") {
                        Some(CoreValue::String(value)) => value.clone(),
                        _ => String::new(),
                    };
                    let propertyName = match descriptor.get("propertyName") {
                        Some(CoreValue::String(value)) => value.clone(),
                        _ => String::new(),
                    };
                    let args = descriptor
                        .get("args")
                        .cloned()
                        .unwrap_or_else(CoreValue::emptyMap);
                    if !streamId.is_empty() && !propertyName.is_empty() {
                        streams.push(operit_link::CoreStreamDescriptor {
                            streamId,
                            target,
                            propertyName,
                            args,
                        });
                    }
                }
            }
            for item in fields.values().take(MAX_STREAM_SCAN_ITEMS) {
                collectMessageStreamsBounded(item, streams, depth + 1);
            }
        }
        _ => {}
    }
}

fn openMessageStreams(session: &Arc<ChatSession>, descriptors: Vec<operit_link::CoreStreamDescriptor>) {
    let active = descriptors
        .iter()
        .map(|descriptor| descriptor.streamId.as_str())
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
    for descriptor in descriptors {
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
        let task = spawnNodeUiSubscription(move |mut cancelled| async move {
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
                // Reopen through the projection so XML and tool payloads stay on Core.
                CoreValue::String("chatMessagesWindowFlow".into()),
            );
            args.insert(
                operit_link::CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT.into(),
                CoreValue::String("watch".into()),
            );
            args.insert(
                operit_link::CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT.into(),
                windowArgs(&session.chatId, *session.window.borrow()),
            );
            let result = openUiSubscription(&cancelled, session
                .client
                .watch(CoreWatchRequest::new(
                    operit_link::nextCoreRouteRequestId("openCoreStream"),
                    descriptor.target,
                    descriptor.propertyName,
                    CoreValue::Map(args),
                )))
                .await;
            match result {
                Ok(None) => return,
                Ok(Some(mut stream)) => {
                    let mut completed = false;
                    while let Some(event) = nextUiEvent(&mut stream, &mut cancelled).await {
                        if event.kind == CoreEventKind::Completed {
                            completed = true;
                            break;
                        }
                        if let Some(text) = session
                            .streams
                            .lock()
                            .unwrap()
                            .get_mut(&descriptor.streamId)
                        {
                            text.apply(&serde_json::to_value(event.value).unwrap_or_default());
                            UI_REVISION.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    if !completed && !*cancelled.borrow() {
                        session.needsReconnect.store(true, Ordering::Release);
                    }
                }
                Err(error) => {
                    session.needsReconnect.store(true, Ordering::Release);
                    *session.error.lock().unwrap() = Some(error.to_string());
                    UI_REVISION.fetch_add(1, Ordering::Relaxed);
                }
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
        let mut tasks = owner.streamTasks.lock().unwrap();
        if owner.retired.load(Ordering::Acquire) { task.abort(); }
        else { tasks.insert(streamId, task); }
    }
}

static UI_REVISION: AtomicU32 = AtomicU32::new(1);
static SESSION: OnceLock<Mutex<Option<Arc<ChatSession>>>> = OnceLock::new();
// Display capacity follows the physical renderer. Encoded protocol state still
// retains all list positions so subsequent deltas are never rebased/truncated.
const MAX_CHAT_MESSAGES: usize = 12;
#[cfg(not(target_os = "espidf"))]
const MAX_CONVERSATIONS: usize = 24;
// Approximately three 320x240 chat viewports; all updates stay volatile.
const MAX_CHAT_STRING_BYTES: usize = 640;
const MAX_CHAT_LINES: usize = 15; // Three five-line chat viewports; 640 UTF-8 bytes also bound wrapped paragraphs.
const MAX_CORE_BYTES: usize = 1024;
const MAX_CORE_VALUE_DEPTH: usize = 16;

fn boundedText(value: &str) -> String {
    if value.len() <= MAX_CHAT_STRING_BYTES {
        return value.to_string();
    }
    let mut end = MAX_CHAT_STRING_BYTES;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn appendTail(target: &mut String, value: &str, limit: usize) {
    let combined = target.len().saturating_add(value.len());
    if combined > limit {
        let remove = combined - limit;
        if remove >= target.len() {
            target.clear();
            let mut start = value.len().saturating_sub(limit);
            while !value.is_char_boundary(start) { start += 1; }
            target.push_str(&value[start..]); return;
        }
        let mut start = remove;
        while !target.is_char_boundary(start) { start += 1; }
        target.drain(..start);
    }
    target.push_str(value);
}

fn appendBounded(target: &mut String, value: &str, limit: usize) {
    if target.len() >= limit {
        return;
    }
    let remaining = limit - target.len();
    let mut end = value.len().min(remaining);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    target.push_str(&value[..end]);
}

// Presentation delimiters are generated by the chat-owned Core projection.
// An incomplete token at a bounded page/stream edge must never become prose.
fn toolTokenName(token: &str) -> Option<&str> {
    let bytes = token.as_bytes();
    if bytes.len() < 5 || bytes[0] != 0x1e || bytes[2] != b'|' ||
        !matches!(bytes[1], b'R' | b'S' | b'F' | b'U') || !token.ends_with('\x1f') {
        return None;
    }
    let name = &token[3..token.len() - 1];
    (name.len() <= 96 && !name.chars().any(char::is_control)).then_some(name)
}
fn completeDisplayTokens(text: &str) -> String {
    let mut rest = text;
    if let Some(end) = rest.find('\x1f') {
        if rest.find('\x1e').is_none_or(|start| start > end) { rest = &rest[end + 1..]; }
    }
    let mut output = String::new();
    while let Some(at) = rest.find('\x1e') {
        output.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find('\x1f') else { return output; };
        let token = &rest[..end + 1];
        if toolTokenName(token).is_some() { output.push_str(token); }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    output
}

fn visibleEdgeText(source: &str) -> String {
    let mut result = String::new();
    let mut rest = source;
    while !rest.is_empty() && result.len() < MAX_CHAT_STRING_BYTES {
        let Some(open) = rest.find('<') else {
            appendBounded(&mut result, rest, MAX_CHAT_STRING_BYTES);
            break;
        };
        appendBounded(&mut result, &rest[..open], MAX_CHAT_STRING_BYTES);
        rest = &rest[open..];
        let Some(close) = rest.find('>') else {
            break;
        };
        let tag = &rest[1..close];
        let name = tag
            .trim_start_matches('/')
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        if name == "link" && (tag.contains("type=\"image\"") || tag.contains("type='image'")) {
            let after_tag = &rest[close + 1..];
            let Some(end) = after_tag.find("</link>") else {
                break;
            };
            appendBounded(&mut result, &rest[..=close], MAX_CHAT_STRING_BYTES);
            appendBounded(&mut result, "</link>", MAX_CHAT_STRING_BYTES);
            rest = &after_tag[end + "</link>".len()..];
            continue;
        }
        if name == "tool" && !tag.starts_with('/') {
            let tool = tag
                .split_once("name=\"")
                .and_then(|(_, tail)| tail.split_once('"').map(|(name, _)| name))
                .or_else(|| {
                    tag.split_once("name='")
                        .and_then(|(_, tail)| tail.split_once('\'').map(|(name, _)| name))
                });
            if let Some(tool) = tool.filter(|name| !name.is_empty()) {
                if !result.is_empty() {
                    appendBounded(&mut result, "\n", MAX_CHAT_STRING_BYTES);
                }
                let name: String = tool.chars().filter(|c| !c.is_control() && *c != '<' && *c != '>').take(96).collect();
                let name = boundedText(&name);
                let mut end = name.len().min(96);
                while !name.is_char_boundary(end) { end -= 1; }
                appendBounded(&mut result, &format!("\x1eR|{}\x1f\n", &name[..end]), MAX_CHAT_STRING_BYTES);
            }
        }
        rest = &rest[close + 1..];
        if !tag.starts_with('/') && !name.is_empty() {
            let closing = format!("</{name}>");
            if let Some(end) = rest.find(&closing) {
                rest = &rest[end + closing.len()..];
            } else if !tag.trim_end().ends_with('/') {
                break;
            }
        }
    }
    result
}

/// Bounds payload fields before they are retained by the long-lived chat state.
/// Collection shape is preserved so subsequent Core deltas still address valid paths.
fn boundCoreValue(value: &mut CoreValue, depth: usize) {
    if depth > MAX_CORE_VALUE_DEPTH {
        *value = CoreValue::Null;
        return;
    }
    match value {
        CoreValue::String(text) => {
            if text.len() > MAX_CHAT_STRING_BYTES {
                let bounded = boundedText(text);
                *text = bounded;
            }
        }
        CoreValue::Bytes(bytes) => bytes.truncate(MAX_CORE_BYTES),
        CoreValue::List(items) => {
            for item in items {
                boundCoreValue(item, depth + 1);
            }
        }
        CoreValue::Map(fields) => {
            for item in fields.values_mut() {
                boundCoreValue(item, depth + 1);
            }
        }
        _ => {}
    }
}

fn mapString<'a>(fields: &'a BTreeMap<String, CoreValue>, key: &str) -> &'a str {
    match fields.get(key) {
        Some(CoreValue::String(value)) => value,
        _ => "",
    }
}

#[cfg(not(target_os = "espidf"))]
fn simpleJson(value: Option<&CoreValue>) -> serde_json::Value {
    match value {
        Some(CoreValue::String(value)) => boundedText(value).into(),
        Some(CoreValue::Bool(value)) => (*value).into(),
        Some(CoreValue::Signed(value)) => (*value).into(),
        Some(CoreValue::Unsigned(value)) => (*value).into(),
        Some(CoreValue::Float(value)) => (*value).into(),
        _ => serde_json::Value::Null,
    }
}

/// Called on the authenticated Link runtime after Space provisions the chat.
pub fn install(
    client: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    services: NodeServices,
    chatId: String,
) {
    let provision = chatId.trim().is_empty();
    let chatId = if provision { uuid::Uuid::new_v4().to_string() } else { chatId };
    installSession(client, services, chatId, provision);
}

fn installSession(
    client: Arc<dyn CoreLinkSharedClient + Send + Sync>, services: NodeServices,
    chatId: String, provision: bool,
) {
    clear();
    let (window, _) = tokio::sync::watch::channel(None);
    let session = Arc::new(ChatSession {
        spaceIdentity: services.peers().spaceConnection().map(|c| c.identity),
        client,
        services,
        chatId,
        ready: AtomicBool::new(!provision),
        retired: AtomicBool::new(false),
        needsReconnect: AtomicBool::new(false),
        histories: Mutex::new(UiValue::default()),
        tasks: Mutex::new(Vec::new()),
        messages: Mutex::new(UiValue::default()),
        window, older: Mutex::new(None), windowHistory: Mutex::new(Vec::new()),
        error: Mutex::new(None),
        executionError: Mutex::new(None),
        generating: AtomicBool::new(false),
        sending: AtomicBool::new(false),
        sendResult: Mutex::new(None),
        subscriptionsClosed: AtomicBool::new(false),
        streams: Mutex::new(BTreeMap::new()),
        streamTasks: Mutex::new(BTreeMap::new()),
        plugins: Mutex::new(PluginUiState::default()),
    });
    *SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(session.clone());
    if provision {
        let owner = session.clone();
        let task = spawnNodeUiSubscription(move |cancelled| async move {
            if *cancelled.borrow() { return; }
            // Idempotent explicit creation. Never cancel the in-flight call or
            // turn an ordinary read/send into implicit object creation.
            let result = session.client.call(CoreCallRequest::new(
                operit_link::nextCoreRouteRequestId("ensureRoutedChat"), CORE_INTERNAL_TARGET,
                "ensureRoutedChat", operit_link::toCoreValue(serde_json::json!({"chatId":session.chatId})).unwrap(),
            )).await.result;
            if *cancelled.borrow() || !currentSession(&session) { return; }
            match result {
                Ok(_) => {
                    session.ready.store(true, Ordering::Release);
                    startSubscriptions(session);
                }
                Err(error) => {
                    *session.error.lock().unwrap() = Some(format!("初始化远端对话失败：{error}"));
                    session.needsReconnect.store(true, Ordering::Release);
                }
            }
            UI_REVISION.fetch_add(1, Ordering::Relaxed);
        });
        owner.addTask(task);
    } else { startSubscriptions(session); }
}

fn startSubscriptions(session: Arc<ChatSession>) {
    if PLUGINS_REQUESTED.load(Ordering::Acquire) { let _ = pluginsAction("edge_plugins_refresh"); }
    let mut pageChanges = session.window.subscribe();
    let owner = session.clone();
    #[cfg(not(target_os = "espidf"))]
    let historySession = session.clone();
    let stateSession = session.clone();
    let task = spawnNodeUiSubscription(move |mut cancelled| async move {
        let _lifetime = SubscriptionLifetime { session: session.clone(), cancelled: cancelled.clone() };
        loop {
            let cursor = *pageChanges.borrow_and_update();
            let result = openUiSubscription(&cancelled, watchChatMessages(&session.client, &session.chatId, cursor)).await;
            let mut stream = match result {
                Ok(None) => return,
                Ok(Some(stream)) => stream,
                Err(error) => { *session.error.lock().unwrap() = Some(error.to_string()); UI_REVISION.fetch_add(1, Ordering::Relaxed); return; }
            };
            if *pageChanges.borrow() != cursor { drop(stream); continue; }
            let mut page = CoreValue::Null;
            let mut switched = false;
            loop {
                let event = tokio::select! {
                    biased;
                    _ = pageChanges.changed() => { switched = true; None }
                    event = nextUiEvent(&mut stream, &mut cancelled) => event,
                };
                let Some(event) = event else { break; };
                if event.kind == CoreEventKind::Completed { break; }
                let next = if event.kind == CoreEventKind::Delta {
                    std::mem::replace(&mut page, CoreValue::Null).intoIncrementalDelta(&event.value)
                } else { Ok(event.value) };
                let result = next.and_then(|value| {
                    page = value;
                    let CoreValue::Map(fields) = &page else { return Err("Expected a bounded chat window".into()); };
                    if let Some(CoreValue::String(error)) = fields.get("error") { return Err(error.clone()); }
                    let Some(CoreValue::List(items)) = fields.get("messages") else { return Err("Missing chat window messages".into()); };
                    if items.len() > MAX_CHAT_MESSAGES { return Err("Chat window exceeds the negotiated row limit".into()); }
                    let older = fields.get("older").and_then(|value| {
                        let value: serde_json::Value = operit_link::fromCoreValue(value.clone()).ok()?;
                        Some(PageCursor { timestamp:value["timestamp"].as_i64()?, offset:value["offset"].as_u64()?.try_into().ok()? })
                    });
                    *session.older.lock().unwrap() = older;
                    let mut messages = session.messages.lock().unwrap();
                    messages.replace(fields.get("messages").cloned().ok_or("Missing chat window messages")?)?;
                    let mut rows = Vec::new(); let mut descriptors = Vec::new();
                    messages.forEachLast(MAX_CHAT_MESSAGES, |item| {
                        collectMessageStreams(&item, &mut descriptors);
                        if let serde_json::Value::Array(mut projected) = projectMessages(&CoreValue::List(vec![item])) { rows.append(&mut projected); }
                    })?;
                    messages.display = serde_json::Value::Array(rows);
                    openMessageStreams(&session, descriptors);
                    *session.error.lock().unwrap() = None;
                    Ok(())
                });
                if let Err(error) = result {
                    *session.error.lock().unwrap() = Some(error); UI_REVISION.fetch_add(1, Ordering::Relaxed); return;
                }
                UI_REVISION.fetch_add(1, Ordering::Relaxed);
            }
            drop(stream);
            for (_, task) in std::mem::take(&mut *session.streamTasks.lock().unwrap()) { task.abort(); }
            session.streams.lock().unwrap().clear();
            if !switched {
                recordSubscriptionEnd(&session.subscriptionsClosed, &cancelled);
                if !*cancelled.borrow() { *session.error.lock().unwrap() = Some("聊天连接已断开，请等待重新连接".into()); UI_REVISION.fetch_add(1, Ordering::Relaxed); }
                return;
            }
        }
    });
    owner.addTask(task);
    // The standalone device UI has no conversation shelf/selector. Do not
    // subscribe to 24 unused history rows on ESP32; simulator owns that surface.
    #[cfg(not(target_os = "espidf"))]
    {
    let task = spawnNodeUiSubscription(move |mut cancelled| async move {
        let session = historySession;
        let _lifetime = SubscriptionLifetime { session: session.clone(), cancelled: cancelled.clone() };
        let result = openUiSubscription(&cancelled, session
            .client
            .watch(CoreWatchRequest::new(
                operit_link::nextCoreRouteRequestId("routedChatListFlow"),
                CORE_INTERNAL_TARGET,
                "routedChatListFlow",
                operit_link::toCoreValue(serde_json::json!({"chatId":session.chatId})).unwrap(),
            )))
            .await;
        match result {
            Ok(None) => return,
            Ok(Some(mut stream)) => {
                while let Some(event) = nextUiEvent(&mut stream, &mut cancelled).await {
                    if event.kind == CoreEventKind::Completed {
                        break;
                    }
                    let mut histories = session.histories.lock().unwrap();
                    let result = histories.update(event).and_then(|()| {
                        let mut rows = Vec::new();
                        histories.forEachLast(MAX_CONVERSATIONS, |item| {
                            if let serde_json::Value::Array(mut projected) = displayConversations(&CoreValue::List(vec![item])) {
                                rows.append(&mut projected);
                            }
                        })?;
                        let start = rows.len().saturating_sub(MAX_CONVERSATIONS);
                        histories.display = serde_json::Value::Array(rows.split_off(start));
                        Ok(())
                    });
                    if let Err(error) = result {
                        *session.error.lock().unwrap() = Some(error);
                        break;
                    }
                    UI_REVISION.fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(error) if isMissingSpaceRoute(&error) => {}
            Err(error) => {
                *session.error.lock().unwrap() = Some(format!("对话列表加载失败：{}", error));
                UI_REVISION.fetch_add(1, Ordering::Relaxed);
            }
        }
    });
    owner.addTask(task);
    }
    let task = spawnNodeUiSubscription(move |mut cancelled| async move {
        let session = stateSession;
        let _lifetime = SubscriptionLifetime { session: session.clone(), cancelled: cancelled.clone() };
        let result = openUiSubscription(&cancelled, session.client.watch(CoreWatchRequest::new(
            operit_link::nextCoreRouteRequestId("edge-chat-state"), CORE_INTERNAL_TARGET,
            "chatStateFlow", operit_link::toCoreValue(serde_json::json!({"chatId": session.chatId})).unwrap(),
        ))).await;
        match result {
            Ok(None) => return,
            Ok(Some(mut stream)) => {
                let mut state = CoreValue::Null;
                while let Some(event) = nextUiEvent(&mut stream, &mut cancelled).await {
                    if event.kind == CoreEventKind::Completed { break; }
                    let next = if event.kind == CoreEventKind::Delta {
                        std::mem::replace(&mut state, CoreValue::Null).intoIncrementalDelta(&event.value)
                    } else { Ok(event.value) };
                    match next {
                        Ok(mut value) => {
                            boundCoreValue(&mut value, 0);
                            state = value;
                            let (generating, error) = executionStatus(&state);
                            session.generating.store(generating, Ordering::Release);
                            *session.executionError.lock().unwrap() = error;
                            UI_REVISION.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(error) => {
                            *session.executionError.lock().unwrap() = Some(error);
                            UI_REVISION.fetch_add(1, Ordering::Relaxed);
                            return;
                        }
                    }
                }
                recordSubscriptionEnd(&session.subscriptionsClosed, &cancelled);
                session.generating.store(false, Ordering::Release);
                if session.executionError.lock().unwrap().is_none() {
                    *session.executionError.lock().unwrap() = Some("聊天状态连接已断开".into());
                }
                UI_REVISION.fetch_add(1, Ordering::Relaxed);
            }
            Err(error) => {
                *session.executionError.lock().unwrap() = Some(error.to_string());
                UI_REVISION.fetch_add(1, Ordering::Relaxed);
            }
        }
    });
    owner.addTask(task);
}

/// Keep only the execution summary for the device UI; model execution stays on Core.
fn executionStatus(value: &CoreValue) -> (bool, Option<String>) {
    let CoreValue::Map(fields) = value else { return (false, None) };
    let generating = matches!(fields.get("isLoading"), Some(CoreValue::Bool(true)));
    let error = fields.get("inputProcessingState").and_then(|v| match v {
        CoreValue::Map(v) => v.get("Error"), _ => None,
    }).and_then(|v| match v {
        CoreValue::Map(v) => v.get("message"), _ => None,
    }).and_then(|v| match v { CoreValue::String(s) => Some({ let mut text = String::new(); appendBounded(&mut text, s, MAX_CHAT_STRING_BYTES); text }), _ => None });
    (generating, error)
}

// Peer reachability can recover before the 500 ms route poll observes an
// outage. A completed long-lived chat watch must still replace its old UI
// session, even when spaceClient() already points at the reconnected peer.
fn recordSubscriptionEnd(closed: &AtomicBool, cancelled: &tokio::sync::watch::Receiver<bool>) {
    if !*cancelled.borrow() { closed.store(true, Ordering::Release); }
}

pub fn needsReconnect() -> bool {
    SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap().as_ref()
        .is_some_and(|session| session.needsReconnect.load(Ordering::Acquire)
            || session.subscriptionsClosed.load(Ordering::Acquire))
}

/// Returns the live Space route state, rather than whether the pairing
/// listener is enabled.
pub fn isConnected() -> bool {
    SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap().as_ref()
        .is_some_and(|session| !session.subscriptionsClosed.load(Ordering::Acquire)
            && session.services.peers().spaceConnection().is_some())
}

/// Returns a monotonic version for display consumers that repaint on change.
pub fn revision() -> u32 {
    UI_REVISION.load(Ordering::Relaxed)
}

/// Drops the local chat route and display state. Pairing credentials are
/// cleared separately by the Edge pairing authority.
pub fn clear() {
    UI_REVISION.fetch_add(1, Ordering::Relaxed);
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|mut session| session.take());
    if let Some(session) = session {
        session.retired.store(true, Ordering::Release);
        for task in std::mem::take(&mut *session.tasks.lock().unwrap()) {
            task.abort();
        }
        for (_, task) in std::mem::take(&mut *session.streamTasks.lock().unwrap()) {
            task.abort();
        }
    }
}

static PLUGINS_REQUESTED: AtomicBool = AtomicBool::new(false);
static PLUGINS_EXCLUSIVE: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct PluginUiState {
    items: Vec<serde_json::Value>, total: u32, offset: u32,
    loading: bool, error: Option<String>, generation: u64,
    testing: bool, batchId: String, tested: u32, failed: u32,
    details: serde_json::Value, detailGeneration: u64,
}

pub fn pluginsSnapshot() -> serde_json::Value {
    let session = SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap().clone();
    let Some(session) = session.filter(|s| s.ready.load(Ordering::Acquire)) else {
        return serde_json::json!({"items":[],"total":0,"offset":0,"loading":false,"error":"等待 Core 连接","category":if PLUGINS_EXCLUSIVE.load(Ordering::Acquire){"exclusive"}else{"general"}});
    };
    let state = session.plugins.lock().unwrap();
    serde_json::json!({"items":state.items,"total":state.total,"offset":state.offset,"loading":state.loading,"error":state.error,
        "testing":state.testing,"tested":state.tested,"failed":state.failed,
        "category":if PLUGINS_EXCLUSIVE.load(Ordering::Acquire) {"exclusive"} else {"general"},"details":state.details})
}

// All execution still goes to the selected chat's Core Binding. Only the two
// fixed plugin diagnostic functions are reachable, not arbitrary tool invokes.
pub fn pluginsAction(action: &str) -> Result<(), String> {
    PLUGINS_REQUESTED.store(true, Ordering::Release);
    let session = SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap().clone()
        .filter(|s| s.ready.load(Ordering::Acquire)).ok_or("等待 Core 连接")?;
    if let Some(id) = action.strip_prefix("edge_plugin_open:") {
        if !session.plugins.lock().unwrap().items.iter().any(|item| item["id"] == id) {
            return Err("插件列表已变化，请刷新".into());
        }
        return startPluginDetails(session, id.to_owned(), 0);
    }
    if action == "edge_plugin_close" {
        let mut state = session.plugins.lock().unwrap();
        state.detailGeneration += 1; state.details = serde_json::Value::Null;
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }
    if action == "edge_plugin_tools_next" || action == "edge_plugin_tools_prev" {
        let (id, offset) = {
            let state = session.plugins.lock().unwrap();
            if state.details["loading"] == true { return Ok(()); }
            let id = state.details["id"].as_str().ok_or("请重新打开插件")?.to_owned();
            let current = state.details["toolOffset"].as_u64().unwrap_or(0) as u32;
            let total = state.details["toolTotal"].as_u64().unwrap_or(0) as u32;
            (id, if action.ends_with("next") { if current + 3 < total {current + 3} else {current} } else {current.saturating_sub(3)})
        };
        return startPluginDetails(session, id, offset);
    }
    if action == "edge_plugins_test_all" { return startPluginBatch(session); }
    if let Some(id) = action.strip_prefix("edge_plugin_probe:") {
        return startPluginTest(session, id, false);
    }
    if let Some(id) = action.strip_prefix("edge_plugin_tool_test:") {
        return startPluginTest(session, id, true);
    }
    let (offset, generation) = {
        let mut state = session.plugins.lock().unwrap();
        if state.loading || state.testing { return Ok(()); }
        let offset = match action {
            "edge_plugins_next" => if state.offset + 6 < state.total { state.offset + 6 } else { state.offset },
            "edge_plugins_prev" => state.offset.saturating_sub(6),
            "edge_plugins_refresh" => state.offset,
            "edge_plugins_exclusive" | "edge_plugins_general" => {
                PLUGINS_EXCLUSIVE.store(action == "edge_plugins_exclusive", Ordering::Release); 0
            }
            _ => return Err("Unknown plugin page action".into()),
        };
        state.loading = true; state.error = None; state.generation += 1;
        state.details = serde_json::Value::Null; state.detailGeneration += 1;
        (offset, state.generation)
    };
    let category = if PLUGINS_EXCLUSIVE.load(Ordering::Acquire) {"exclusive"} else {"general"};
    UI_REVISION.fetch_add(1, Ordering::Relaxed);
    let deadlineSession = session.clone();
    spawnNodeUiSubscription(move |_| async move {
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
        if !currentSession(&deadlineSession) { return; }
        let mut state = deadlineSession.plugins.lock().unwrap();
        if state.generation == generation && state.loading {
            state.loading = false; state.error = Some("插件列表读取超时，请刷新".into());
            state.generation += 1; UI_REVISION.fetch_add(1, Ordering::Relaxed);
        }
    });
    spawnNodeUiSubscription(move |_| async move {
        let result = session.client.call(CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId("chatAvailablePlugins"), CORE_INTERNAL_TARGET,
            "chatAvailablePlugins", operit_link::toCoreValue(serde_json::json!({"chatId":session.chatId,"offset":offset,"category":category})).unwrap(),
        )).await.result;
        if !currentSession(&session) { return; }
        let mut state = session.plugins.lock().unwrap();
        if state.generation != generation { return; }
        state.loading = false;
        match result {
            Ok(value) => {
                let value = serde_json::to_value(value).unwrap_or_default();
                state.total = value["total"].as_u64().unwrap_or(0).min(u32::MAX as u64) as u32;
                state.offset = offset;
                state.items = value["items"].as_array().into_iter().flatten().take(6).filter_map(|item| {
                    let id = item["id"].as_str()?;
                    if id.len() > 108 { return None; }
                    let text = item["name"].as_str().unwrap_or(id);
                    let mut end = text.len().min(42);
                    while !text.is_char_boundary(end) { end -= 1; }
                    Some(serde_json::json!({"id":id,"name":text[..end],
                        "available":item["available"] == true,"status":"untested","latencyMs":null,
                        "toolStatus":"untested","toolLatencyMs":null,"testError":""}))
                }).collect();
            }
            Err(error) => { state.items.clear(); state.error = Some(boundedText(&error.to_string())); }
        }
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
    });
    Ok(())
}
// Single selected, bounded metadata projection. Closing/changing tabs invalidates
// old replies; deadline drains the underlying Link call just like diagnostics.
fn startPluginDetails(session: Arc<ChatSession>, id: String, offset: u32) -> Result<(), String> {
    let generation = {
        let mut state = session.plugins.lock().unwrap();
        if state.loading || !state.items.iter().any(|item| item["id"] == id) { return Err("插件列表已变化".into()); }
        state.detailGeneration += 1;
        state.details = serde_json::json!({"id":id,"loading":true,"tools":[],"toolTotal":0,"toolOffset":offset});
        state.detailGeneration
    };
    UI_REVISION.fetch_add(1, Ordering::Relaxed);
    spawnNodeUiSubscription(move |_| async move {
        let reply = pluginCallDeadline(&session, CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId("chatPluginDetails"), CORE_INTERNAL_TARGET,"chatPluginDetails",
            operit_link::toCoreValue(serde_json::json!({"chatId":session.chatId,"packageName":id,"toolOffset":offset})).unwrap(),
        )).await;
        if !currentSession(&session) {return;}
        let mut state = session.plugins.lock().unwrap();
        if state.detailGeneration != generation {return;}
        let value = match reply {
            Ok(value) => serde_json::to_value(value).unwrap_or_default(),
            Err(_) => serde_json::json!({"error":"插件详情读取失败或超时"}),
        };
        let text = |text: &str, max: usize| {
            let clean: String = text.chars().filter(|c| !c.is_control()).collect();
            let mut end = clean.len().min(max);
            while !clean.is_char_boundary(end) {end -= 1;}
            clean[..end].to_owned()
        };
        state.details = serde_json::json!({"id":id,"loading":false,
            "description":text(value["description"].as_str().unwrap_or(""),192),
            "tools":value["tools"].as_array().into_iter().flatten().take(3)
                .filter_map(|tool| tool.as_str().map(|s|text(s,64))).collect::<Vec<_>>(),
            "toolOffset":value["toolOffset"].as_u64().unwrap_or(offset as u64).min(u32::MAX as u64),
            "toolTotal":value["toolTotal"].as_u64().unwrap_or(0).min(u32::MAX as u64),
            "error":text(value["error"].as_str().unwrap_or(""),80)});
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
    });
    Ok(())
}
fn preparePluginTest(session: &Arc<ChatSession>, id: &str, probeId: &str, tool: bool) {
    let mut state = session.plugins.lock().unwrap();
    if let Some(item) = state.items.iter_mut().find(|item| item["id"] == id) {
        let (status, request, latency) = if tool { ("toolStatus", "toolProbeId", "toolLatencyMs") } else { ("status", "probeId", "latencyMs") };
        item[status] = "probing".into(); item[request] = probeId.into();
        item[latency] = serde_json::Value::Null; item["testError"] = "".into();
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
    }
}
async fn pluginCallDeadline(session: &Arc<ChatSession>, request: CoreCallRequest) -> Result<CoreValue, String> {
    pluginCallDeadlineWithLimit(session, request, std::time::Duration::from_secs(8)).await
}
async fn pluginCallDeadlineWithLimit(session: &Arc<ChatSession>, request: CoreCallRequest, limit: std::time::Duration) -> Result<CoreValue, String> {
    let client = session.client.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    // A UI deadline must never drop an ambiguous in-flight Link transaction:
    // Duplex deliberately closes the session if that happens. Drain its ack
    // on a separate task; timed-out results cannot mutate a newer UI attempt.
    spawnNodeUiSubscription(move |_| async move {
        let response = client.call(request).await;
        let _ = sender.send(response.result);
    });
    match tokio::time::timeout(limit, receiver).await {
        Ok(Ok(result)) => result.map_err(|e| e.to_string()),
        Ok(Err(_)) => Err("插件测试连接已关闭".into()),
        Err(_) => Err("插件测试超时".into()),
    }
}
async fn callPluginTest(session: &Arc<ChatSession>, id: &str, probeId: &str, tool: bool) -> (bool, u64, String) {
    let started = std::time::Instant::now();
    let result = pluginCallDeadline(session, CoreCallRequest::new(
        probeId, CORE_INTERNAL_TARGET, "chatPluginStatus",
        operit_link::toCoreValue(serde_json::json!({"chatId":session.chatId,"packageName":id,
            "testKind":if tool { "tool" } else { "connection" }})).unwrap(),
    )).await;
    let (ok, error) = match result {
        Ok(value) => {
            let value = serde_json::to_value(value).unwrap_or_default();
            (value["success"] == true, value["message"].as_str().unwrap_or("插件测试结果无效").to_owned())
        },
        Err(error) => (false, error),
    };
    (ok, started.elapsed().as_millis().min(99999) as u64, error)
}
fn startPluginTest(session: Arc<ChatSession>, id: &str, tool: bool) -> Result<(), String> {
    let generation;
    {
        let state = session.plugins.lock().unwrap();
        if state.testing || state.loading { return Ok(()); }
        let item = state.items.iter().find(|item| item["id"] == id).ok_or("插件列表已变化，请刷新")?;
        if item["status"] == "probing" || item["toolStatus"] == "probing" { return Ok(()); }
        generation = state.generation;
    }
    let id = id.to_owned(); let probeId = operit_link::nextCoreRouteRequestId("chatPluginStatus");
    preparePluginTest(&session, &id, &probeId, tool);
    spawnNodeUiSubscription(move |_| async move {
        let (ok, ms, error) = callPluginTest(&session, &id, &probeId, tool).await;
        finishPluginTest(&session, generation, &id, &probeId, tool, ok, ms, &error);
    });
    Ok(())
}
fn finishPluginTest(session: &Arc<ChatSession>, generation: u64, id: &str, probeId: &str, tool: bool, ok: bool, ms: u64, error: &str) {
    if !currentSession(session) { return; }
    let mut state = session.plugins.lock().unwrap();
    if state.generation != generation { return; }
    let (status, request, latency) = if tool { ("toolStatus", "toolProbeId", "toolLatencyMs") } else { ("status", "probeId", "latencyMs") };
    if let Some(item) = state.items.iter_mut().find(|item| item["id"] == id && item[status] == "probing" && item[request] == probeId) {
        item[status] = if ok { "success" } else { "failure" }.into();
        item[latency] = ms.into(); item["testError"] = boundedText(error).into();
        if !tool { item["error"] = boundedText(error).into(); }
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(test)]
fn finishPluginProbe(session: &Arc<ChatSession>, generation: u64, id: &str, probeId: &str, ok: bool, ms: u64, error: &str) {
    finishPluginTest(session, generation, id, probeId, false, ok, ms, error);
}
fn startPluginBatch(session: Arc<ChatSession>) -> Result<(), String> {
    let category = if PLUGINS_EXCLUSIVE.load(Ordering::Acquire) {"exclusive"} else {"general"};
    let (generation, batchId) = {
        let mut state = session.plugins.lock().unwrap();
        if state.testing || state.loading { return Ok(()); }
        if state.items.iter().any(|item| item["status"] == "probing" || item["toolStatus"] == "probing") {
            return Err("请等待当前测试完成".into());
        }
        state.testing = true; state.tested = 0; state.failed = 0; state.error = None;
        state.batchId = operit_link::nextCoreRouteRequestId("pluginBatch");
        (state.generation, state.batchId.clone())
    };
    UI_REVISION.fetch_add(1, Ordering::Relaxed);
    spawnNodeUiSubscription(move |_| async move {
        let mut offset = 0;
        let mut error = None;
        // Fetch one bounded page at a time, never retain the complete package catalog.
        'pages: loop {
            if !pluginBatchCurrent(&session, generation, &batchId) { return; }
            let reply = pluginCallDeadline(&session, CoreCallRequest::new(
                operit_link::nextCoreRouteRequestId("chatAvailablePlugins"), CORE_INTERNAL_TARGET, "chatAvailablePlugins",
                operit_link::toCoreValue(serde_json::json!({"chatId":session.chatId,"offset":offset,"category":category})).unwrap(),
            )).await;
            let page = match reply {
                Ok(value) => serde_json::to_value(value).unwrap_or_default(),
                Err(e) => { error = Some(boundedText(&e)); break; }
            };
            let items = page["items"].as_array().map(Vec::as_slice).unwrap_or(&[]);
            if items.is_empty() { break; }
            for item in items.iter().take(6) {
                if !pluginBatchCurrent(&session, generation, &batchId) { return; }
                let Some(id) = item["id"].as_str().filter(|id| id.len() <= 108) else {
                    error = Some("插件标识超出设备限制".into()); break 'pages;
                };
                let probeId = operit_link::nextCoreRouteRequestId("chatPluginStatus");
                preparePluginTest(&session, id, &probeId, false);
                let (ok, ms, message) = callPluginTest(&session, id, &probeId, false).await;
                if !pluginBatchCurrent(&session, generation, &batchId) { return; }
                finishPluginTest(&session, generation, id, &probeId, false, ok, ms, &message);
                let mut state = session.plugins.lock().unwrap();
                state.tested += 1; if !ok { state.failed += 1; }
                UI_REVISION.fetch_add(1, Ordering::Relaxed);
            }
            offset += items.len().min(6) as u32;
            if offset >= page["total"].as_u64().unwrap_or(0) as u32 { break; }
        }
        if !pluginBatchCurrent(&session, generation, &batchId) { return; }
        let mut state = session.plugins.lock().unwrap(); state.testing = false; state.error = error;
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
    });
    Ok(())
}
fn pluginBatchCurrent(session: &Arc<ChatSession>, generation: u64, batchId: &str) -> bool {
    if !currentSession(session) { return false; }
    let state = session.plugins.lock().unwrap();
    state.generation == generation && state.testing && state.batchId == batchId
}

pub fn snapshot() -> serde_json::Value {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone();
    match session {
        Some(session) => {
            // Do not hold UI state locks while serializing or acquiring another
            // lock: the single-threaded network runtime also updates this state.
            let streams = session.streams.lock().unwrap().clone();
            let conversations = session.histories.lock().unwrap().display.clone();
            let messages = renderMessages(session.messages.lock().unwrap().display.clone(), &streams);
            let error = session.error.lock().unwrap().clone()
                .or_else(|| session.executionError.lock().unwrap().clone());
            serde_json::json!({
                "connected": session.services.peers().spaceConnection().is_some(), "chatId": session.chatId,
                "initializing": !session.ready.load(Ordering::Acquire),
                "conversations": conversations,
                "hasOlder": session.older.lock().unwrap().is_some(),
                "hasNewer": !session.windowHistory.lock().unwrap().is_empty(),
                "sending": session.sending.load(Ordering::Acquire),
                "generating": session.generating.load(Ordering::Acquire),
                "messages": messages, "error": error,
            })
        }
        None => {
            serde_json::json!({"connected": false, "messages": [], "error": "等待同一 Space 的 Core 建立聊天连接"})
        }
    }
}

/// The title comes from conversation metadata, never from a response body.
#[cfg(not(target_os = "espidf"))]
pub fn preview() -> String {
    let value = snapshot();
    activeConversation(&value)
        .and_then(|chat| chat["characterCardName"].as_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Operit")
        .to_string()
}

fn activeConversation(value: &serde_json::Value) -> Option<&serde_json::Value> {
    value["conversations"]
        .as_array()?
        .iter()
        .find(|chat| chat["id"] == value["chatId"])
}

/// Changes only this device's view; the full Core continues owning the chat.
pub fn selectChat(id: &str) -> Result<(), String> {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()
        .ok_or("设备尚未连接")?;
    if session
        .services
        .peers()
        .activePeerNodeIds()
        .unwrap_or_default()
        .is_empty()
    {
        return Err("设备已离线".into());
    }
    let exists = session.histories.lock().unwrap().display.as_array()
        .is_some_and(|items| items.iter().any(|item| item["id"] == id));
    if !exists {
        return Err("对话已不存在，请刷新列表".into());
    }
    install(
        session.client.clone(),
        session.services.clone(),
        id.to_string(),
    );
    Ok(())
}

/// Move between bounded display pages. Only tiny cursors survive page eviction.
pub fn moveHistory(older: bool) -> Result<(), String> {
    let session = SESSION.get_or_init(|| Mutex::new(None)).lock().unwrap().clone().ok_or("设备尚未连接")?;
    let mut history = session.windowHistory.lock().unwrap();
    let cursor = if older {
        let Some(cursor) = *session.older.lock().unwrap() else { return Ok(()); };
        let current = *session.window.borrow();
        if current == Some(cursor) { return Ok(()); }
        if history.len() == 32 { history.remove(0); }
        history.push(current); Some(cursor)
    } else { history.pop().unwrap_or(None) };
    if *session.window.borrow() != cursor { session.window.send_replace(cursor); }
    Ok(())
}

pub fn newChat() -> Result<(), String> {
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()
        .ok_or("设备尚未连接")?;
    if session
        .services
        .peers()
        .activePeerNodeIds()
        .unwrap_or_default()
        .is_empty()
    {
        return Err("设备已离线".into());
    }
    if !session.ready.load(Ordering::Acquire) { return Err("远端对话正在初始化，请稍后重试".into()); }
    if session.sending.swap(true, Ordering::AcqRel) {
        return Err("请等待当前操作完成".into());
    }
    let value = snapshot();
    let current = activeConversation(&value).cloned().unwrap_or_default();
    spawnNodeUiTask(move || async move {
        let response = session
            .client
            .call(CoreCallRequest::new(
                operit_link::nextCoreRouteRequestId("createRoutedChat"),
                CORE_INTERNAL_TARGET,
                "createRoutedChat",
                operit_link::toCoreValue(serde_json::json!({
                    "chatId":session.chatId, "characterCardName":current["characterCardName"],
                    "group":current["group"], "characterGroupId":current["characterGroupId"],
                }))
                .unwrap(),
            ))
            .await;
        session.sending.store(false, Ordering::Release);
        match response.result {
            Ok(CoreValue::String(id)) if !id.trim().is_empty() => {
                let current = SESSION.get().unwrap().lock().unwrap().clone();
                if current
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &session))
                {
                    install(session.client.clone(), session.services.clone(), id);
                }
            }
            result => {
                *session.error.lock().unwrap() = Some(match result {
                    Err(error) => error.to_string(),
                    _ => "创建对话返回了无效结果".into(),
                });
                UI_REVISION.fetch_add(1, Ordering::Relaxed);
            }
        }
    });
    Ok(())
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
    if session.error.lock().unwrap().is_some() || session.executionError.lock().unwrap().is_some() {
        return "错误".into();
    }
    if session
        .services
        .peers()
        .activePeerNodeIds()
        .unwrap_or_default()
        .is_empty()
    {
        return "离线".into();
    }
    if session.sending.load(Ordering::Acquire) {
        return "发送中".into();
    }
    if session.generating.load(Ordering::Acquire) { return "生成中".into(); }
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

/// Empty/error state only; actual messages are rendered as individual rows.
#[cfg(not(target_os = "espidf"))]
pub fn screenText() -> String {
    let value = snapshot();
    screenTextFromSnapshot(&value)
}

pub fn screenTextFromSnapshot(value: &serde_json::Value) -> String {
    if let Some(error) = value["error"].as_str() {
        return error.to_string();
    }
    if value["messages"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        return String::new();
    }
    if isConnected() {
        "输入消息开始对话".into()
    } else {
        "已离线，等待重新连接".into()
    }
}

#[cfg(test)]
fn displayMessages(value: &CoreValue, streams: &BTreeMap<String, StreamText>) -> serde_json::Value {
    renderMessages(projectMessages(value), streams)
}

// Cache only display rows for the UI thread. It must not decode a second full
// message tree concurrently with the UART reader on this small device.
fn projectMessages(value: &CoreValue) -> serde_json::Value {
    let CoreValue::List(items) = value else {
        return serde_json::json!([]);
    };
    let start = items.len().saturating_sub(MAX_CHAT_MESSAGES);
    let mut result = Vec::with_capacity(items.len() - start);
    for item in &items[start..] {
        let CoreValue::Map(fields) = item else {
            continue;
        };
        let text = boundedText(mapString(fields, "text"));
        let streamId = match fields.get("contentStream") {
            Some(CoreValue::Map(content)) => match content.get("$coreStream") {
                Some(CoreValue::Map(descriptor)) => mapString(descriptor, "streamId"),
                _ => "",
            },
            _ => "",
        };
        let text = completeDisplayTokens(&displayText(&text));
        if text.trim().is_empty() && streamId.is_empty() {
            continue;
        }
        result.push(serde_json::json!({
            "sender": mapString(fields, "sender"),
            "text": text,
            "streamId": streamId,
        }));
    }
    serde_json::Value::Array(result)
}

fn renderMessages(mut rows: serde_json::Value, streams: &BTreeMap<String, StreamText>) -> serde_json::Value {
    let Some(items) = rows.as_array_mut() else { return serde_json::json!([]); };
    for row in items.iter_mut() {
        if let Some(stream) = row["streamId"].as_str().and_then(|id| streams.get(id)) {
            if !stream.completed && !stream.text.is_empty() {
                let text = completeDisplayTokens(&displayText(&visibleEdgeText(&stream.text)));
                row["text"] = serde_json::Value::String(text);
            }
        }
        if let Some(fields) = row.as_object_mut() { fields.remove("streamId"); }
    }
    items.retain(|row| !row["text"].as_str().unwrap_or("").trim().is_empty());
    // A live tail replaces its snapshot text, so reapply the total window
    // budget across all rows rather than allowing each stream another window.
    let (mut bytes, mut lines) = (MAX_CHAT_STRING_BYTES, MAX_CHAT_LINES);
    let mut first = items.len();
    for index in (0..items.len()).rev() {
        if bytes == 0 || lines == 0 { break; }
        let text = items[index]["text"].as_str().unwrap_or("");
        let lineStart = text.match_indices('\n').rev().nth(lines - 1).map(|(at, _)| at + 1).unwrap_or(0);
        let mut start = text.len().saturating_sub(bytes).max(lineStart);
        while !text.is_char_boundary(start) { start += 1; }
        if let Some(end) = text[start..].find('\x1f') {
            if text[start..].find('\x1e').is_none_or(|open| open > end) { start += end + 1; }
        }
        let tail = &text[start..];
        bytes = bytes.saturating_sub(tail.len());
        lines = lines.saturating_sub(1 + tail.matches('\n').count());
        if start > 0 { items[index]["text"] = tail.to_owned().into(); }
        first = index;
    }
    items.drain(..first);
    rows
}

#[cfg(not(target_os = "espidf"))]
fn displayConversations(value: &CoreValue) -> serde_json::Value {
    let CoreValue::List(items) = value else {
        return serde_json::json!([]);
    };
    let start = items.len().saturating_sub(MAX_CONVERSATIONS);
    let mut result = Vec::with_capacity(items.len() - start);
    for item in &items[start..] {
        let CoreValue::Map(fields) = item else {
            continue;
        };
        result.push(serde_json::json!({
            "id": boundedText(mapString(fields, "id")),
            "title": boundedText(mapString(fields, "title")),
            "characterCardName": boundedText(mapString(fields, "characterCardName")),
            "group": simpleJson(fields.get("group")),
            "characterGroupId": simpleJson(fields.get("characterGroupId")),
        }));
    }
    serde_json::Value::Array(result)
}

/// Uploads one already bounded image as binary Link data, then sends its Core-owned media link.
#[cfg(not(target_os = "espidf"))]
pub fn sendImage(bytes: Vec<u8>, mime: String) -> Result<(), String> {
    if bytes.is_empty()
        || bytes.len() > 512 * 1024
        || !matches!(mime.as_str(), "image/png" | "image/jpeg")
    {
        return Err("只支持小于 512 KiB 的 PNG/JPEG 图片".into());
    }
    let session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .clone()
        .ok_or("设备尚未连接")?;
    if session
        .services
        .peers()
        .activePeerNodeIds()
        .unwrap_or_default()
        .is_empty()
    {
        return Err("设备已离线".into());
    }
    if !session.ready.load(Ordering::Acquire) { return Err("远端对话正在初始化，请稍后重试".into()); }
    if session.sending.swap(true, Ordering::AcqRel) {
        return Err("请等待当前消息发送完成".into());
    }
    spawnNodeUiTask(move || async move {
        let result = async {
            let args = operit_link::CoreValue::Map(std::collections::BTreeMap::from([
                ("chatId".into(), CoreValue::String(session.chatId.clone())),
                ("mimeType".into(), CoreValue::String(mime)),
                ("imageBytes".into(), CoreValue::Bytes(bytes)),
            ]));
            let response = session
                .client
                .call(CoreCallRequest::new(
                    operit_link::nextCoreRouteRequestId("registerChatImage"),
                    CORE_INTERNAL_TARGET,
                    "registerChatImage",
                    args,
                ))
                .await;
            let link = match response.result.map_err(|e| e.to_string())? {
                CoreValue::String(link) if link.len() <= 100 => link,
                _ => return Err("Core 未返回有效的图片引用".into()),
            };
            if !link.starts_with("<link type=\"image\" id=\"") {
                return Err("Core 返回了无效图片引用".into());
            }
            sendImageMessage(&session, link).await
        }
        .await;
        if let Err(error) = &result {
            *session.error.lock().unwrap() = Some(error.clone());
        }
        *session.sendResult.lock().unwrap() = Some(result);
        session.sending.store(false, Ordering::Release);
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
    });
    Ok(())
}

/// Bridges a synchronous firmware callback to its existing Link runtime.
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
    if session
        .services
        .peers()
        .activePeerNodeIds()
        .unwrap_or_default()
        .is_empty()
    {
        return Err("设备已离线".into());
    }
    if !session.ready.load(Ordering::Acquire) { return Err("远端对话正在初始化，请稍后重试".into()); }
    if session.sending.swap(true, Ordering::AcqRel) {
        return Err("上一条消息仍在发送".into());
    }
    *session.error.lock().unwrap() = None;
    spawnNodeUiTask(move || async move {
        let result = sendImageMessage(&session, text).await;
        if let Err(error) = &result {
            *session.error.lock().unwrap() = Some(error.clone());
        }
        *session.sendResult.lock().unwrap() = Some(result);
        session.sending.store(false, Ordering::Release);
        UI_REVISION.fetch_add(1, Ordering::Relaxed);
    });
    Ok(())
}

async fn sendImageMessage(session: &ChatSession, text: String) -> Result<(), String> {
    let args = operit_link::toCoreValue(serde_json::json!({
        "promptFunctionType": "CHAT", "roleCardIdOverride": null,
        "chatIdOverride": session.chatId, "messageText": text,
        "proxySenderNameOverride": null, "chatProviderIdOverride": null,
        "chatModelIdOverride": null, "attachments": [], "replyToMessage": null,
        "turnOptions": {"persistTurn": true, "notifyReply": null, "hideUserMessage": false,
            "disableWarning": false, "chatInputSubmitRequestedHandled": false},
    }))
    .unwrap();
    session
        .client
        .call(CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId("sendUserMessage"),
            CORE_INTERNAL_TARGET,
            "sendUserMessage",
            args,
        ))
        .await
        .result
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_reply_keeps_a_bounded_utf8_tail_without_freezing() {
        let mut text=StreamText::default();
        for _ in 0..1000 { text.apply(&serde_json::json!({"type":"chunk","value":"增量同步的文字"})); assert!(text.text.len()<=MAX_CHAT_STRING_BYTES); }
        text.apply(&serde_json::json!({"type":"chunk","value":"最新的结束标记"}));
        assert!(text.text.ends_with("最新的结束标记"));
        assert!(text.text.is_char_boundary(text.text.len()));
        let CoreValue::Map(args)=windowArgs("chat",Some(PageCursor{timestamp:123,offset:456})) else {panic!()};
        assert_eq!(args["textBytes"],CoreValue::Unsigned(MAX_CHAT_STRING_BYTES as u64));
        assert_eq!(args["textLines"],CoreValue::Unsigned(MAX_CHAT_LINES as u64));
        for n in 0..100 {
            text.apply(&serde_json::json!({"type":"chunk","value":format!("\n短行-{n}")}));
            assert!(1 + text.text.matches('\n').count() <= MAX_CHAT_LINES);
        }
        assert!(text.text.ends_with("短行-99"));
    }

    #[test]
    fn subscription_end_requires_reconnect_but_local_cancellation_does_not() {
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        let closed = AtomicBool::new(false);
        recordSubscriptionEnd(&closed, &cancelled);
        assert!(closed.load(Ordering::Acquire));
        closed.store(false, Ordering::Release);
        cancel.send_replace(true);
        recordSubscriptionEnd(&closed, &cancelled);
        assert!(!closed.load(Ordering::Acquire));
    }

    fn uiEvent(kind: CoreEventKind, value: CoreValue) -> operit_link::CoreEvent {
        operit_link::CoreEvent { requestId: None, target: "chat".into(),
            propertyName: "messages".into(), kind, value }
    }

    #[test]
    fn encoded_ui_state_preserves_successive_nested_delta_paths() {
        let value = |count| CoreValue::List((0..count).map(|index| CoreValue::Map(BTreeMap::from([
            ("id".into(), CoreValue::Unsigned(index)),
            ("parts".into(), CoreValue::List(vec![CoreValue::Map(BTreeMap::from([
                ("content".into(), CoreValue::String(format!("reply-{index}"))),
                ("unusedMetadata".into(), CoreValue::Null),
            ]))])),
        ]))).collect());
        let mut cache = UiValue::default();
        let mut previous = None;
        for count in [2, 5, 12, 3, 0, 1] {
            let expected = value(count);
            let (kind, delta) = CoreValue::incrementalEvent(&mut previous, expected.clone());
            cache.update(uiEvent(kind, delta)).unwrap();
            assert_eq!(cache.decode().unwrap(), expected);
        }
    }

    #[test]
    fn item_cache_nested_updates_and_invalid_later_paths_are_transactional() {
        let mut cache = UiValue::default();
        let base = CoreValue::List(vec![CoreValue::Map(BTreeMap::from([
            ("parts".into(), CoreValue::List(vec![CoreValue::String("old".into())])),
            ("untouched".into(), CoreValue::String("same".into())),
        ]))]);
        cache.update(uiEvent(CoreEventKind::Snapshot, base.clone())).unwrap();
        let mut expected = base.clone();
        let CoreValue::List(items) = &mut expected else { panic!("list") };
        let CoreValue::Map(fields) = &mut items[0] else { panic!("map") };
        fields.insert("parts".into(), CoreValue::List(vec![CoreValue::String("new".into()), CoreValue::Null]));
        let mut previous = Some(base);
        let (kind, delta) = CoreValue::incrementalEvent(&mut previous, expected.clone());
        cache.update(uiEvent(kind, delta)).unwrap();
        assert_eq!(cache.decode().unwrap(), expected);
        let operation = |index| CoreValue::Map(BTreeMap::from([
            ("op".into(), CoreValue::String("remove".into())),
            ("path".into(), CoreValue::List(vec![CoreValue::Unsigned(index)])),
        ]));
        let delta = CoreValue::Map(BTreeMap::from([("$coreDelta".into(), CoreValue::List(vec![operation(0), operation(99)]))]));
        assert!(cache.update(uiEvent(CoreEventKind::Delta, delta)).is_err());
        assert_eq!(cache.decode().unwrap(), expected, "no partial update may be published");
    }

    #[test]
    fn cached_display_renders_live_stream_without_decoding_full_state() {
        let messages = CoreValue::List(vec![CoreValue::Map(BTreeMap::from([
            ("sender".into(), CoreValue::String("ai".into())),
            ("text".into(), CoreValue::String(String::new())),
            ("contentStream".into(), CoreValue::Map(BTreeMap::from([
                ("$coreStream".into(), CoreValue::Map(BTreeMap::from([
                    ("streamId".into(), CoreValue::String("live".into())),
                ]))),
            ]))),
        ]))]);
        let projected = projectMessages(&messages);
        assert!(renderMessages(projected.clone(), &BTreeMap::new()).as_array().unwrap().is_empty());
        let streams = BTreeMap::from([("live".into(), StreamText {
            text: "live reply".into(), ..StreamText::default()
        })]);
        let rows = renderMessages(projected, &streams);
        assert_eq!(rows[0]["text"], "live reply");
        assert!(rows[0].get("streamId").is_none(), "private projection metadata must not leak to UI API");
    }

    #[test]
    fn live_stream_and_snapshot_share_one_total_display_budget() {
        let rows = serde_json::json!([
            {"sender":"user", "text":"旧历史\n".repeat(120), "images":[], "streamId":""},
            {"sender":"ai", "text":"", "images":[], "streamId":"live"},
        ]);
        let mut stream = StreamText::default();
        stream.apply(&serde_json::json!({"type":"chunk", "value":"新的回复\n".repeat(120)+"最新终点"}));
        let rows = renderMessages(rows, &BTreeMap::from([("live".into(), stream)]));
        let rows = rows.as_array().unwrap();
        let bytes: usize = rows.iter().map(|r| r["text"].as_str().unwrap().len()).sum();
        let lines: usize = rows.iter().map(|r| 1+r["text"].as_str().unwrap().matches('\n').count()).sum();
        assert!(bytes <= MAX_CHAT_STRING_BYTES); assert!(lines <= MAX_CHAT_LINES);
        assert!(rows.last().unwrap()["text"].as_str().unwrap().ends_with("最新终点"));
    }

    #[test]
    fn invalid_ui_delta_keeps_last_encoded_value() {
        let mut cache = UiValue::default();
        let expected = CoreValue::List(vec![CoreValue::String("kept".into())]);
        cache.update(uiEvent(CoreEventKind::Snapshot, expected.clone())).unwrap();
        let encoded = cache.encoded.clone();
        assert!(cache.update(uiEvent(CoreEventKind::Delta, CoreValue::Null)).is_err());
        assert_eq!(cache.encoded, encoded);
        assert_eq!(cache.decode().unwrap(), expected);
    }

    #[tokio::test]
    async fn cancelled_watch_open_finishes_ack_then_closes_only_that_watch() {
        use std::{future::{poll_fn, Future}, sync::atomic::AtomicUsize, task::Poll};
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        let (ack, reply) = tokio::sync::oneshot::channel::<()>();
        let abandoned = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        struct InFlight(Arc<AtomicUsize>, bool);
        impl Drop for InFlight {
            fn drop(&mut self) {
                if !self.1 { self.0.fetch_add(1, Ordering::SeqCst); }
            }
        }
        let abandoned_open = abandoned.clone();
        let closed_watch = closed.clone();
        let mut opening = Box::pin(openUiSubscription(&cancelled, async move {
            let mut request = InFlight(abandoned_open, false);
            reply.await.unwrap();
            request.1 = true;
            let (_sender, stream) = operit_link::CoreEventStream::channel();
            Ok(stream.withOnClose(move || { closed_watch.fetch_add(1, Ordering::SeqCst); }))
        }));
        poll_fn(|cx| { assert!(opening.as_mut().poll(cx).is_pending()); Poll::Ready(()) }).await;
        cancel.send(true).unwrap(); // Final message removes the in-flight descriptor.
        poll_fn(|cx| { assert!(opening.as_mut().poll(cx).is_pending()); Poll::Ready(()) }).await;
        assert_eq!(abandoned.load(Ordering::SeqCst), 0, "never abandon an in-flight Link transaction");
        ack.send(()).unwrap();
        assert!(opening.await.unwrap().is_none());
        assert_eq!(abandoned.load(Ordering::SeqCst), 0);
        assert_eq!(closed.load(Ordering::SeqCst), 1, "release the late watch exactly once");
    }

    #[tokio::test]
    async fn already_cancelled_subscription_does_not_open_remote_watch() {
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        cancel.send(true).unwrap();
        assert!(openUiSubscription(&cancelled, async { panic!("must not open cancelled watch") }).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn idle_watch_cancellation_stops_receiving_and_releases_stream() {
        let (cancel, mut cancelled) = tokio::sync::watch::channel(false);
        let closed = Arc::new(AtomicBool::new(false));
        let on_close = closed.clone();
        let (_sender, stream) = operit_link::CoreEventStream::channel();
        let mut stream = stream.withOnClose(move || { on_close.store(true, Ordering::Release); });
        cancel.send(true).unwrap();
        assert!(nextUiEvent(&mut stream, &mut cancelled).await.is_none());
        drop(stream);
        assert!(closed.load(Ordering::Acquire));
    }


    #[test]
    fn async_execution_failure_is_visible_and_clears_on_next_run() {
        let failure = operit_link::toCoreValue(serde_json::json!({
            "isLoading": false, "inputProcessingState": {"Error": {"message": "401 Unauthorized"}}
        })).unwrap();
        assert_eq!(executionStatus(&failure), (false, Some("401 Unauthorized".into())));
        assert_eq!(screenTextFromSnapshot(&serde_json::json!({
            "error": executionStatus(&failure).1, "messages": [{"text":"sent"}]
        })), "401 Unauthorized");
        let loading = operit_link::toCoreValue(serde_json::json!({
            "isLoading": true, "inputProcessingState": {"Connecting": {"message":"connecting"}}
        })).unwrap();
        assert_eq!(executionStatus(&loading), (true, None));
        let idle = operit_link::toCoreValue(serde_json::json!({"isLoading":false,"inputProcessingState":"Idle"})).unwrap();
        assert_eq!(executionStatus(&idle), (false, None));
        let big = operit_link::toCoreValue(serde_json::json!({"inputProcessingState":{"Error":{"message":"错".repeat(4000)}}})).unwrap();
        let message = executionStatus(&big).1.unwrap();
        assert!(message.len() <= MAX_CHAT_STRING_BYTES);
        assert!(message.is_char_boundary(message.len()));
    }
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

/// The fixed renderer is text-only. Preserve image-only messages as visible
/// placeholders instead of stripping links into an invisible preview list.
fn displayText(text: &str) -> String {
    let mut output = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<link ") {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find('>') else { break; };
        let header = &rest[6..end];
        if header.split_whitespace().any(|part| part == "type=\"image\"" || part == "type='image'") {
            output.push_str("[图片：请在 Core 端查看]");
            rest = &rest[end + 1..];
            rest = rest.strip_prefix("</link>").unwrap_or(rest);
        } else {
            output.push_str(&rest[..end + 1]);
            rest = &rest[end + 1..];
        }
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
mod attachment_display_tests {
    use super::*;
    #[test]
    fn text_only_renderer_keeps_image_messages_visible() {
        assert_eq!(displayText("<link type=\"image\" id=\"a\"></link>"), "[图片：请在 Core 端查看]");
        assert_eq!(displayText("前<link id='a' type='image'></link>后"), "前[图片：请在 Core 端查看]后");
        assert_eq!(displayText("<link type=\"file\" id=\"a\"></link>"), "<link type=\"file\" id=\"a\"></link>");
        assert_eq!(displayText("前<link type=\"image\""), "前<link type=\"image\"");
    }
}


#[cfg(all(test, not(target_os = "espidf")))]
#[path = "edge_chat_session_tests.rs"]
mod session_tests;

#[cfg(test)]
mod plugin_card_tests {
    use super::*;
    #[test]
    fn live_tool_status_replaces_pending_card_and_survives_savepoint_rollback() {
        let mut stream = StreamText::default();
        stream.apply(&serde_json::json!({"type":"chunk","value":"\x1eR|daily_life:get_current_date\x1f\n"}));
        stream.apply(&serde_json::json!({"type":"savepoint","id":"before-result"}));
        stream.apply(&serde_json::json!({"type":"toolStatus","value":"\x1eS|daily_life:get_current_date\x1f"}));
        assert_eq!(stream.text.matches('\x1e').count(), 1);
        assert!(stream.text.contains("\x1eS|"));
        stream.apply(&serde_json::json!({"type":"rollback","id":"before-result"}));
        assert!(stream.text.contains("\x1eR|"));
        stream.apply(&serde_json::json!({"type":"toolStatus","value":"\x1eF|daily_life:get_current_date\x1f"}));
        assert!(stream.text.contains("\x1eF|"));
        stream.apply(&serde_json::json!({"type":"toolStatus","value":"\x1eS|wrong_tool\x1f"}));
        assert!(!stream.text.contains("wrong_tool"));
        stream.apply(&serde_json::json!({"type":"reset"})); assert!(stream.text.is_empty());
    }
    #[test]
    fn completed_stream_yields_to_final_core_status_and_bounded_tokens_remain_complete() {
        let mut streams = BTreeMap::new();
        streams.insert("s".into(), StreamText {text:"\x1eR|date\x1f".into(), completed:true, ..Default::default()});
        let rows = renderMessages(serde_json::json!([{"sender":"ai","text":"\x1eS|date\x1f","streamId":"s"}]), &streams);
        assert_eq!(rows[0]["text"], "\x1eS|date\x1f");
        assert_eq!(completeDisplayTokens("cut-name\x1f\nreply"), "\nreply");
        assert_eq!(completeDisplayTokens("reply\n\x1eR|partial"), "reply\n");
        let large = format!("{}\x1eR|{}\x1f\n{}", "a".repeat(600), "x".repeat(96), "b".repeat(600));
        let rows = renderMessages(serde_json::json!([{"sender":"ai","text":large}]), &BTreeMap::new());
        let text = rows[0]["text"].as_str().unwrap();
        assert!(text.len() <= MAX_CHAT_STRING_BYTES);
        assert_eq!(text.matches('\x1e').count(), text.matches('\x1f').count());
    }
}
