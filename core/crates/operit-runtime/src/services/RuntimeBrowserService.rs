use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::HostManager::HostManager;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{
    BrowserSessionCommand, BrowserSessionCommandResult, BrowserSessionHost, BrowserSessionInfo,
    BrowserSessionSnapshot,
};
use operit_util::stream::HotStream::MutableSharedStreamImpl;
use operit_util::stream::Stream::{CollectFuture, Stream};
use operit_util::AppLogger::AppLogger;
use serde::{Deserialize, Serialize};

const BROWSER_RUNTIME_LOG_TAG: &str = "RuntimeBrowser";
const BROWSER_RUNTIME_INTERACTION_ACTION: &str = "interact";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Published browser session metadata for attached views.
pub struct RuntimeBrowserSessionInfo {
    pub sessionId: String,
    pub currentUrl: String,
    pub title: String,
    pub userAgent: Option<String>,
    pub active: bool,
    pub canGoBack: bool,
    pub canGoForward: bool,
    pub isLoading: bool,
    pub progress: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Semantic browser command submitted by a remote controller.
pub struct RuntimeBrowserCommand {
    pub action: String,
    pub sessionId: Option<String>,
    pub url: Option<String>,
    pub script: Option<String>,
    pub payloadJson: String,
    pub userAgent: Option<String>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Result returned after the owner host executes a browser command.
pub struct RuntimeBrowserCommandResult {
    pub success: bool,
    pub session: Option<RuntimeBrowserSessionInfo>,
    pub sessions: Vec<RuntimeBrowserSessionInfo>,
    pub resultJson: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Latest browser session state used when a controller attaches.
pub struct RuntimeBrowserSessionSnapshot {
    pub session: RuntimeBrowserSessionInfo,
    pub resultJson: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Browser event payload published by the owner host.
pub struct RuntimeBrowserSessionEvent {
    pub sessionId: String,
    pub eventType: String,
    pub session: Option<RuntimeBrowserSessionInfo>,
    pub resultJson: String,
    #[serde(default, with = "serde_bytes")]
    pub frameData: Vec<u8>,
    pub frameCodec: Option<String>,
    pub frameWidth: Option<i32>,
    pub frameHeight: Option<i32>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// One framed browser event emitted over the attached Link stream.
pub struct RuntimeBrowserStreamEvent {
    pub sequence: u64,
    pub sessionId: String,
    pub eventType: String,
    pub session: Option<RuntimeBrowserSessionInfo>,
    pub resultJson: String,
    #[serde(default, with = "serde_bytes")]
    pub frameData: Vec<u8>,
    pub frameCodec: Option<String>,
    pub frameWidth: Option<i32>,
    pub frameHeight: Option<i32>,
    pub error: Option<String>,
}

/// Facade over owner-host browser sessions and their Link event streams.
pub struct RuntimeBrowserService {
    browserSessionHost: Arc<dyn BrowserSessionHost>,
}

#[derive(Clone, Debug)]
/// Stream wrapper exposing browser events to attached remote controllers.
pub struct RuntimeBrowserEventStream {
    upstream: MutableSharedStreamImpl<RuntimeBrowserStreamEvent>,
}

impl RuntimeBrowserEventStream {
    /// Wraps a shared browser event stream.
    pub fn new(upstream: MutableSharedStreamImpl<RuntimeBrowserStreamEvent>) -> Self {
        Self { upstream }
    }
}

impl Stream for RuntimeBrowserEventStream {
    type Item = RuntimeBrowserStreamEvent;

    /// Collects serialized browser events from the shared stream.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        self.upstream.collect(collector)
    }
}

static BROWSER_EVENT_STREAMS: OnceLock<
    Mutex<HashMap<String, MutableSharedStreamImpl<RuntimeBrowserStreamEvent>>>,
> = OnceLock::new();
static BROWSER_EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Returns the process-wide browser event stream registry.
fn browser_event_streams(
) -> &'static Mutex<HashMap<String, MutableSharedStreamImpl<RuntimeBrowserStreamEvent>>> {
    BROWSER_EVENT_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns the shared stream for one browser session.
fn browser_event_stream(sessionId: &str) -> MutableSharedStreamImpl<RuntimeBrowserStreamEvent> {
    let mut streams = browser_event_streams()
        .lock()
        .expect("browser event streams mutex poisoned");
    streams
        .entry(sessionId.to_string())
        .or_insert_with(|| MutableSharedStreamImpl::new(0))
        .clone()
}

/// Publishes one owner-host browser event to attached controllers.
fn publish_browser_event(event: RuntimeBrowserSessionEvent) -> Result<(), String> {
    let sessionId = event.sessionId.clone();
    let framed = RuntimeBrowserStreamEvent {
        sequence: BROWSER_EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        sessionId,
        eventType: event.eventType,
        session: event.session,
        resultJson: event.resultJson,
        frameData: event.frameData,
        frameCodec: event.frameCodec,
        frameWidth: event.frameWidth,
        frameHeight: event.frameHeight,
        error: event.error,
    };
    browser_event_stream(&framed.sessionId).emit(framed);
    Ok(())
}

/// Builds a runtime command with common empty fields.
fn runtime_browser_command(action: &str) -> RuntimeBrowserCommand {
    RuntimeBrowserCommand {
        action: action.to_string(),
        sessionId: None,
        url: None,
        script: None,
        payloadJson: String::new(),
        userAgent: None,
        headers: BTreeMap::new(),
    }
}

/// Converts a host session into the runtime browser session model.
fn runtime_browser_session_info(session: BrowserSessionInfo) -> RuntimeBrowserSessionInfo {
    RuntimeBrowserSessionInfo {
        sessionId: session.sessionId,
        currentUrl: session.currentUrl,
        title: session.title,
        userAgent: session.userAgent,
        active: session.active,
        canGoBack: session.canGoBack,
        canGoForward: session.canGoForward,
        isLoading: session.isLoading,
        progress: session.progress,
    }
}

/// Converts an optional host session into the runtime browser model.
fn runtime_browser_session_info_option(
    session: Option<BrowserSessionInfo>,
) -> Option<RuntimeBrowserSessionInfo> {
    session.map(runtime_browser_session_info)
}

/// Converts host sessions into runtime browser session models.
fn runtime_browser_session_infos(
    sessions: Vec<BrowserSessionInfo>,
) -> Vec<RuntimeBrowserSessionInfo> {
    sessions
        .into_iter()
        .map(runtime_browser_session_info)
        .collect()
}

/// Converts a runtime browser command into a host browser command.
fn host_browser_session_command(command: RuntimeBrowserCommand) -> BrowserSessionCommand {
    BrowserSessionCommand {
        action: command.action,
        sessionId: command.sessionId,
        url: command.url,
        script: command.script,
        payloadJson: command.payloadJson,
        userAgent: command.userAgent,
        headers: command.headers,
    }
}

/// Converts a host command result into the runtime browser command model.
fn runtime_browser_command_result(
    result: BrowserSessionCommandResult,
) -> RuntimeBrowserCommandResult {
    RuntimeBrowserCommandResult {
        success: result.success,
        session: runtime_browser_session_info_option(result.session),
        sessions: runtime_browser_session_infos(result.sessions),
        resultJson: result.resultJson,
        error: result.error,
    }
}

/// Converts a host browser snapshot into the runtime snapshot model.
fn runtime_browser_session_snapshot(
    snapshot: BrowserSessionSnapshot,
) -> RuntimeBrowserSessionSnapshot {
    RuntimeBrowserSessionSnapshot {
        session: runtime_browser_session_info(snapshot.session),
        resultJson: snapshot.resultJson,
    }
}

impl RuntimeBrowserService {
    /// Creates the browser service facade.
    #[allow(non_snake_case)]
    pub fn getInstance(context: &HostManager) -> Self {
        Self {
            browserSessionHost: context
                .browserSessionHost
                .clone()
                .expect("BrowserSessionHost must be configured for RuntimeBrowserService"),
        }
    }

    /// Lists browser sessions currently owned by the owner host.
    #[allow(non_snake_case)]
    pub fn listBrowserSessions(&self) -> Result<Vec<RuntimeBrowserSessionInfo>, String> {
        self.browserSessionHost
            .listBrowserSessions()
            .map(runtime_browser_session_infos)
            .map_err(|error| error.message)
    }

    /// Creates a browser session on the owner host.
    #[allow(non_snake_case)]
    pub fn createBrowserSession(
        &self,
        initialUrl: String,
        userAgent: Option<String>,
        headers: BTreeMap<String, String>,
    ) -> Result<RuntimeBrowserSessionInfo, String> {
        let session = self
            .browserSessionHost
            .createBrowserSession(&initialUrl, userAgent.as_deref(), headers)
            .map(runtime_browser_session_info)
            .map_err(|error| error.message)?;
        browser_event_stream(&session.sessionId);
        publish_browser_event(RuntimeBrowserSessionEvent {
            sessionId: session.sessionId.clone(),
            eventType: "created".to_string(),
            session: Some(session.clone()),
            resultJson: String::new(),
            frameData: Vec::new(),
            frameCodec: None,
            frameWidth: None,
            frameHeight: None,
            error: None,
        })?;
        Ok(session)
    }

    /// Updates browser session metadata on the owner host.
    #[allow(non_snake_case)]
    pub fn updateBrowserSession(
        &self,
        sessionId: String,
        userAgent: Option<String>,
        headers: BTreeMap<String, String>,
    ) -> Result<RuntimeBrowserSessionInfo, String> {
        self.browserSessionHost
            .updateBrowserSession(&sessionId, userAgent.as_deref(), headers)
            .map(runtime_browser_session_info)
            .map_err(|error| error.message)
    }

    /// Returns the shared event stream used by an attached browser controller.
    #[allow(non_snake_case)]
    pub fn browserSessionEvents(&self, sessionId: String) -> RuntimeBrowserEventStream {
        RuntimeBrowserEventStream::new(browser_event_stream(&sessionId))
    }

    /// Submits one semantic browser command to the owner host.
    #[allow(non_snake_case)]
    pub fn submitBrowserCommand(
        &self,
        command: RuntimeBrowserCommand,
    ) -> Result<RuntimeBrowserCommandResult, String> {
        let startedAtMillis = currentTimeMillis();
        let log_context = (command.action != BROWSER_RUNTIME_INTERACTION_ACTION)
            .then(|| (command.action.clone(), command.sessionId.clone()));
        if let Some((action, session_id)) = log_context.as_ref() {
            AppLogger::i(
                BROWSER_RUNTIME_LOG_TAG,
                &format!("command start action={} session={:?}", action, session_id),
            );
        }
        let result = self
            .browserSessionHost
            .submitBrowserCommand(host_browser_session_command(command))
            .map(runtime_browser_command_result)
            .map_err(|error| error.message)?;
        if let Some((action, session_id)) = log_context.as_ref() {
            AppLogger::i(
                BROWSER_RUNTIME_LOG_TAG,
                &format!(
                    "command done action={} session={:?} elapsedMs={}",
                    action,
                    session_id,
                    currentTimeMillis() - startedAtMillis
                ),
            );
        }
        Ok(result)
    }

    /// Returns the latest browser session snapshot for a controller.
    #[allow(non_snake_case)]
    pub fn getBrowserSessionSnapshot(
        &self,
        sessionId: String,
    ) -> Result<RuntimeBrowserSessionSnapshot, String> {
        self.browserSessionHost
            .getBrowserSessionSnapshot(&sessionId)
            .map(runtime_browser_session_snapshot)
            .map_err(|error| error.message)
    }

    /// Publishes one browser session event from the owner host.
    #[allow(non_snake_case)]
    pub fn publishBrowserSessionEvent(
        &self,
        event: RuntimeBrowserSessionEvent,
    ) -> Result<(), String> {
        publish_browser_event(event)
    }

    /// Explicitly closes a browser session on the owner host.
    #[allow(non_snake_case)]
    pub fn closeBrowserSession(&self, sessionId: String) -> Result<(), String> {
        let result = self
            .browserSessionHost
            .closeBrowserSession(&sessionId)
            .map(runtime_browser_command_result)
            .map_err(|error| error.message)?;
        publish_browser_event(RuntimeBrowserSessionEvent {
            sessionId: sessionId.clone(),
            eventType: "closed".to_string(),
            session: result.session,
            resultJson: result.resultJson,
            frameData: Vec::new(),
            frameCodec: None,
            frameWidth: None,
            frameHeight: None,
            error: result.error,
        })?;
        let stream = browser_event_streams()
            .lock()
            .expect("browser event streams mutex poisoned")
            .remove(&sessionId);
        if let Some(stream) = stream {
            stream.close();
        }
        Ok(())
    }
}
