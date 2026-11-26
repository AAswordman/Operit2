//! Bounded text display and forwarding over the authenticated Core session.
#![allow(non_snake_case)]

use operit_edge_sdk::contract::chat_display::ChatDisplayEvent;

use operit_link::CoreLinkSharedClient;
use operit_link::{
    CoreCallRequest, CoreEventKind, CoreValue, CoreWatchRequest, CORE_INTERNAL_TARGET,
};
use operit_edge_sdk::runtime::NodeServices::NodeServices;
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

async fn openUiSubscription<F>(
    cancelled: &tokio::sync::watch::Receiver<bool>,
    open: impl FnOnce() -> F,
) -> Result<Option<operit_link::CoreEventStream>, operit_link::CoreLinkError>
where F: std::future::Future<Output = Result<operit_link::CoreEventStream, operit_link::CoreLinkError>> {
    static OPENING: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    if *cancelled.borrow() { return Ok(None); }
    let mut waiting = cancelled.clone();
    let opening = tokio::select! {
        biased;
        _ = uiTaskCancelled(&mut waiting) => return Ok(None),
        opening = OPENING.get_or_init(|| tokio::sync::Mutex::new(())).lock() => opening,
    };
    if *cancelled.borrow() { return Ok(None); }
    // Construct the boxed routing/authorization future only after admission.
    // Await its acknowledgement intact; established streams stay independent.
    let result = open().await;
    drop(opening);
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

struct ChatSession {
    client: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    services: NodeServices,
    tasks: Mutex<Vec<NodeUiTask>>,
    chatId: String,
    ready: AtomicBool,
    retired: AtomicBool,
    needsReconnect: AtomicBool,
    messages: Mutex<UiValue>,
    error: Mutex<Option<String>>,
    executionError: Mutex<Option<String>>,
    generating: AtomicBool,
    sending: AtomicBool,
    sendResult: Mutex<Option<Result<(), String>>>,
    subscriptionsClosed: AtomicBool,
    streams: Mutex<BTreeMap<String, StreamText>>,
    streamTasks: Mutex<BTreeMap<String, NodeUiTask>>,
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
    identity: Option<operit_edge_sdk::runtime::NodeServices::SpaceClientIdentity>,
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

/// Opens the single bounded display route and preserves its protocol error.
async fn watchChatMessages(
    client: &Arc<dyn CoreLinkSharedClient + Send + Sync>,
    chatId: &str,
) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {
    client.watch(CoreWatchRequest::new(
        "edge-ui-messages",
        CORE_INTERNAL_TARGET,
        "chatMessagesWindowFlow",
        windowArgs(chatId),
    )).await
}

// Share these small wire builders instead of constructing and copying a JSON
// tree for each call. Keep one copy in Flash across all UI request paths.
#[inline(never)]
fn chatFields(chatId: &str) -> BTreeMap<String, CoreValue> {
    let mut args = BTreeMap::new();
    args.insert("chatId".into(), CoreValue::String(chatId.into()));
    args
}
fn chatArgs(chatId: &str) -> CoreValue { CoreValue::Map(chatFields(chatId)) }
#[inline(never)]
/// Encodes the device's fixed byte and line budget for one display page.
fn windowArgs(chatId: &str) -> CoreValue {
    let mut args = chatFields(chatId);
    args.insert("beforeTimestamp".into(), CoreValue::Null);
    args.insert("beforeTextOffset".into(), CoreValue::Null);
    args.insert("textBytes".into(), CoreValue::Unsigned(MAX_CHAT_STRING_BYTES as u64));
    args.insert("textLines".into(), CoreValue::Unsigned(MAX_CHAT_LINES as u64));
    CoreValue::Map(args)
}

#[derive(Default, Clone)]
struct StreamText {
    text: String,
    completed: bool,
}

impl StreamText {
    /// Decodes fixtures through the same presentation contract as Link.
    #[cfg(test)]
    fn applyValue(&mut self, value: &serde_json::Value) {
        self.apply(serde::Deserialize::deserialize(value).expect("valid display event")).expect("valid display change");
    }

    /// Applies presentation editing operations without interpreting business messages.
    fn apply(&mut self, event: ChatDisplayEvent) -> Result<(), String> {
        match event {
            ChatDisplayEvent::Chunk { value } => appendTail(&mut self.text, &value, MAX_CHAT_STRING_BYTES),
            ChatDisplayEvent::Replace { value } => { self.text.clear(); appendTail(&mut self.text, &value, MAX_CHAT_STRING_BYTES); }
            ChatDisplayEvent::Error { message } => return Err(message),
        }
        if let Some(start) = self.text.match_indices('\n').rev().nth(MAX_CHAT_LINES - 1).map(|(at, _)| at + 1) {
            self.text.drain(..start);
        }
        Ok(())
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
                windowArgs(&session.chatId),
            );
            let result = openUiSubscription(&cancelled, || session
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
                            match operit_link::fromCoreValue::<ChatDisplayEvent>(event.value) {
                                Ok(event) => { if let Err(error) = text.apply(event) { *session.error.lock().unwrap() = Some(error); break; } },
                                Err(error) => { *session.error.lock().unwrap() = Some(error.to_string()); break; }
                            }
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
    let session = Arc::new(ChatSession {
        client,
        services,
        chatId,
        ready: AtomicBool::new(!provision),
        retired: AtomicBool::new(false),
        needsReconnect: AtomicBool::new(false),
        tasks: Mutex::new(Vec::new()),
        messages: Mutex::new(UiValue::default()),
        error: Mutex::new(None),
        executionError: Mutex::new(None),
        generating: AtomicBool::new(false),
        sending: AtomicBool::new(false),
        sendResult: Mutex::new(None),
        subscriptionsClosed: AtomicBool::new(false),
        streams: Mutex::new(BTreeMap::new()),
        streamTasks: Mutex::new(BTreeMap::new()),
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
                "ensureRoutedChat", chatArgs(&session.chatId),
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
    let owner = session.clone();
    let stateSession = session.clone();
    let task = spawnNodeUiSubscription(move |mut cancelled| async move {
        let _lifetime = SubscriptionLifetime { session: session.clone(), cancelled: cancelled.clone() };
        let result = openUiSubscription(&cancelled, || watchChatMessages(&session.client, &session.chatId)).await;
        let mut stream = match result {
            Ok(None) => return,
            Ok(Some(stream)) => stream,
            Err(error) => { *session.error.lock().unwrap() = Some(error.to_string()); UI_REVISION.fetch_add(1, Ordering::Relaxed); return; }
        };
        let mut page = CoreValue::Null;
        loop {
            let Some(event) = nextUiEvent(&mut stream, &mut cancelled).await else { break; };
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
        recordSubscriptionEnd(&session.subscriptionsClosed, &cancelled);
        if !*cancelled.borrow() {
            *session.error.lock().unwrap() = Some("聊天连接已断开，请等待重新连接".into());
            UI_REVISION.fetch_add(1, Ordering::Relaxed);
        }
    });
    owner.addTask(task);
    let task = spawnNodeUiSubscription(move |mut cancelled| async move {
        let session = stateSession;
        let _lifetime = SubscriptionLifetime { session: session.clone(), cancelled: cancelled.clone() };
        let result = openUiSubscription(&cancelled, || session.client.watch(CoreWatchRequest::new(
            operit_link::nextCoreRouteRequestId("edge-chat-state"), CORE_INTERNAL_TARGET,
            "chatStateFlow", chatArgs(&session.chatId),
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
            let rows = session.messages.lock().unwrap().display.clone();
            let messages = renderMessages(rows, &session.streams.lock().unwrap());
            let error = session.error.lock().unwrap().clone()
                .or_else(|| session.executionError.lock().unwrap().clone());
            serde_json::json!({
                "connected": session.services.peers().spaceConnection().is_some(), "chatId": session.chatId,
                "initializing": !session.ready.load(Ordering::Acquire),
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
    spawnNodeUiTask(move || async move {
        let args = chatFields(&session.chatId);
        let response = session
            .client
            .call(CoreCallRequest::new(
                operit_link::nextCoreRouteRequestId("createRoutedChat"),
                CORE_INTERNAL_TARGET,
                "createRoutedChat",
                CoreValue::Map(args),
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

/// Reads empty and error display text from the already materialized chat snapshot.
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
/// Copies presentation rows without parsing message markup or tool metadata.
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

/// Combines the latest snapshot and stream tail within one shared display budget.
fn renderMessages(mut rows: serde_json::Value, streams: &BTreeMap<String, StreamText>) -> serde_json::Value {
    let Some(items) = rows.as_array_mut() else { return serde_json::json!([]); };
    for row in items.iter_mut() {
        if let Some(stream) = row["streamId"].as_str().and_then(|id| streams.get(id)) {
            if !stream.completed && !stream.text.is_empty() {
                let text = boundedText(&stream.text);
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
        let tail = &text[start..];
        bytes = bytes.saturating_sub(tail.len());
        lines = lines.saturating_sub(1 + tail.matches('\n').count());
        if start > 0 { items[index]["text"] = tail.to_owned().into(); }
        first = index;
    }
    items.drain(..first);
    rows
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
                ("imageBytes".into(), CoreValue::Bytes(bytes.into())),
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
    if text.len() > 16384 { return Err("消息超过 16384 字节，请拆分发送".into()); }
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
    // Construct Link values directly: avoid duplicating the input through a
    // JSON value tree on the device, and keep the firmware serializer small.
    let mut args = BTreeMap::new();
    for key in ["participantId", "proxySenderNameOverride", "chatProviderIdOverride",
        "chatModelIdOverride", "replyToMessage"] {
        args.insert(key.into(), CoreValue::Null);
    }
    args.insert("promptFunctionType".into(), CoreValue::String("CHAT".into()));
    args.insert("chatIdOverride".into(), CoreValue::String(session.chatId.clone()));
    args.insert("attachments".into(), CoreValue::List(Vec::new()));
    let mut options = BTreeMap::new();
    for (key, enabled) in [("persistTurn", true), ("hideUserMessage", false),
        ("disableWarning", false), ("chatInputSubmitRequestedHandled", false)] {
        options.insert(key.into(), CoreValue::Bool(enabled));
    }
    options.insert("notifyReply".into(), CoreValue::Null);
    args.insert("turnOptions".into(), CoreValue::Map(options));
    if text.len() > 2048 {
        args.insert("expectedByteLength".into(), CoreValue::Unsigned(text.len() as u64));
        let mut upload = session.client.openPush(operit_link::CorePushRequest::new(
            operit_link::nextCoreRouteRequestId("sendUserMessageStream"),
            CORE_INTERNAL_TARGET, "sendUserMessageStream").withArgs(CoreValue::Map(args)))
            .await.map_err(|e| e.to_string())?;
        let mut offset = 0;
        while offset < text.len() {
            let mut end = (offset + 2048).min(text.len());
            while !text.is_char_boundary(end) { end -= 1; }
            upload.send(CoreValue::String(text[offset..end].to_owned())).await.map_err(|e| e.to_string())?;
            offset = end;
        }
        upload.send(CoreValue::String(String::new())).await.map_err(|e| e.to_string())?;
        return upload.close().await.map_err(|e| e.to_string());
    }
    args.insert("messageText".into(), CoreValue::String(text));
    session
        .client
        .call(CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId("sendUserMessage"),
            CORE_INTERNAL_TARGET,
            "sendUserMessage",
            CoreValue::Map(args),
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
        for _ in 0..1000 { text.applyValue(&serde_json::json!({"type":"chunk","value":"增量同步的文字"})); assert!(text.text.len()<=MAX_CHAT_STRING_BYTES); }
        text.applyValue(&serde_json::json!({"type":"chunk","value":"最新的结束标记"}));
        assert!(text.text.ends_with("最新的结束标记"));
        assert!(text.text.is_char_boundary(text.text.len()));
        let CoreValue::Map(args)=windowArgs("chat") else {panic!()};
        assert_eq!(args["textBytes"],CoreValue::Unsigned(MAX_CHAT_STRING_BYTES as u64));
        assert_eq!(args["textLines"],CoreValue::Unsigned(MAX_CHAT_LINES as u64));
        for n in 0..100 {
            text.applyValue(&serde_json::json!({"type":"chunk","value":format!("\n短行-{n}")}));
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
        stream.applyValue(&serde_json::json!({"type":"chunk", "value":"新的回复\n".repeat(120)+"最新终点"}));
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
        let mut opening = Box::pin(openUiSubscription(&cancelled, || async move {
            let mut request = InFlight(abandoned_open, false);
            reply.await.unwrap();
            request.1 = true;
            let (_sender, stream) = operit_link::CoreEventStream::channel();
            Ok(stream.withOnClose(move || { closed_watch.fetch_add(1, Ordering::SeqCst); }))
        }));
        poll_fn(|cx| { assert!(opening.as_mut().poll(cx).is_pending()); Poll::Ready(()) }).await;
        // A second open must not construct its boxed routing future while the
        // first acknowledgement is pending, and a cancelled waiter opens nothing.
        let (cancel_waiter, waiter) = tokio::sync::watch::channel(false);
        let constructed = AtomicBool::new(false);
        let mut queued = Box::pin(openUiSubscription(&waiter, || {
            constructed.store(true, Ordering::SeqCst);
            async { Ok(operit_link::CoreEventStream::channel().1) }
        }));
        poll_fn(|cx| { assert!(queued.as_mut().poll(cx).is_pending()); Poll::Ready(()) }).await;
        assert!(!constructed.load(Ordering::SeqCst));
        cancel_waiter.send(true).unwrap();
        assert!(queued.await.unwrap().is_none());
        assert!(!constructed.load(Ordering::SeqCst));
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
        assert!(openUiSubscription(&cancelled, || async { panic!("must not open cancelled watch") }).await.unwrap().is_none());
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
    /// Replaces discarded display text without storing source revisions on the device.
    fn projected_replacement_removes_discarded_text_without_retaining_revision_history() {
        let mut text = StreamText::default();
        for event in [
            serde_json::json!({"type":"replace","value":""}),
            serde_json::json!({"type":"chunk","value":"你好"}),
            serde_json::json!({"type":"chunk","value":"错误分支"}),
            serde_json::json!({"type":"replace","value":"你好"}),
            serde_json::json!({"type":"chunk","value":"，世界"}),
        ] {
            text.applyValue(&event);
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

#[cfg(all(test, not(target_os = "espidf")))]
#[path = "edge_chat_session_tests.rs"]
mod session_tests;
