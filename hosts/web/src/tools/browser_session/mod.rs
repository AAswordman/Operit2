use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use js_sys::JSON;
use operit_host_api::{
    BrowserSessionCommand, BrowserSessionCommandResult, BrowserSessionHost, BrowserSessionInfo,
    BrowserSessionSnapshot, HostError, HostResult,
};
use wasm_bindgen::JsValue;

#[derive(Clone)]
struct WebBrowserSession {
    info: BrowserSessionInfo,
    headers: BTreeMap<String, String>,
    resultJson: String,
}

thread_local! {
    static NEXT_BROWSER_SESSION_ID: Cell<u64> = const { Cell::new(0) };
    static BROWSER_SESSIONS: RefCell<BTreeMap<String, WebBrowserSession>> = const { RefCell::new(BTreeMap::new()) };
}

/// Simulates browser sessions with browser-native navigation and JavaScript evaluation.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebBrowserSessionHost;

impl WebBrowserSessionHost {
    /// Creates the browser session simulator host.
    pub fn new() -> Self {
        Self
    }
}

impl BrowserSessionHost for WebBrowserSessionHost {
    /// Lists all simulated browser sessions.
    fn listBrowserSessions(&self) -> HostResult<Vec<BrowserSessionInfo>> {
        BROWSER_SESSIONS.with(|sessions| {
            Ok(sessions
                .borrow()
                .values()
                .map(|session| session.info.clone())
                .collect())
        })
    }

    /// Creates one simulated browser session.
    fn createBrowserSession(
        &self,
        initialUrl: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo> {
        let sessionId = nextBrowserSessionId();
        let info = BrowserSessionInfo {
            sessionId: sessionId.clone(),
            currentUrl: initialUrl.to_string(),
            title: initialUrl.to_string(),
            userAgent: userAgent.map(str::to_string),
            active: true,
            canGoBack: false,
            canGoForward: false,
            isLoading: false,
            progress: 100,
        };
        BROWSER_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(
                sessionId,
                WebBrowserSession {
                    info: info.clone(),
                    headers,
                    resultJson: String::new(),
                },
            );
        });
        Ok(info)
    }

    /// Updates simulated session request metadata.
    fn updateBrowserSession(
        &self,
        sessionId: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo> {
        BROWSER_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let session = browserSession(&mut sessions, sessionId)?;
            session.info.userAgent = userAgent.map(str::to_string);
            session.headers = headers;
            Ok(session.info.clone())
        })
    }

    /// Executes a supported browser simulator command.
    fn submitBrowserCommand(
        &self,
        command: BrowserSessionCommand,
    ) -> HostResult<BrowserSessionCommandResult> {
        match command.action.as_str() {
            "list" => Ok(browserResult(
                None,
                self.listBrowserSessions()?,
                String::new(),
                None,
            )),
            "create" => {
                let session = self.createBrowserSession(
                    command.url.as_deref().unwrap_or("about:blank"),
                    command.userAgent.as_deref(),
                    command.headers,
                )?;
                Ok(browserResult(
                    Some(session),
                    self.listBrowserSessions()?,
                    String::new(),
                    None,
                ))
            }
            "close" => {
                let sessionId = commandSessionId(&command)?;
                self.closeBrowserSession(&sessionId)
            }
            "navigate" | "load" | "open" => self.navigate(command),
            "reload" | "refresh" | "back" | "forward" | "interact" => {
                let sessionId = commandSessionId(&command)?;
                let session =
                    BROWSER_SESSIONS.with(|sessions| -> HostResult<BrowserSessionInfo> {
                        let mut sessions = sessions.borrow_mut();
                        Ok(browserSession(&mut sessions, &sessionId)?.info.clone())
                    })?;
                Ok(browserResult(
                    Some(session),
                    self.listBrowserSessions()?,
                    "{}".to_string(),
                    None,
                ))
            }
            "evaluate" | "execute_script" | "script" => self.evaluate(command),
            _ => Ok(browserResult(
                None,
                self.listBrowserSessions()?,
                String::new(),
                Some(format!(
                    "browser simulator does not support action: {}",
                    command.action
                )),
            )),
        }
    }

    /// Returns the latest simulated browser session snapshot.
    fn getBrowserSessionSnapshot(&self, sessionId: &str) -> HostResult<BrowserSessionSnapshot> {
        BROWSER_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let session = browserSession(&mut sessions, sessionId)?;
            Ok(BrowserSessionSnapshot {
                session: session.info.clone(),
                resultJson: session.resultJson.clone(),
            })
        })
    }

    /// Closes one simulated browser session.
    fn closeBrowserSession(&self, sessionId: &str) -> HostResult<BrowserSessionCommandResult> {
        let session = BROWSER_SESSIONS.with(|sessions| -> HostResult<BrowserSessionInfo> {
            sessions
                .borrow_mut()
                .remove(sessionId)
                .map(|session| session.info)
                .ok_or_else(|| HostError::new(format!("browser session not found: {sessionId}")))
        })?;
        Ok(browserResult(
            Some(session),
            self.listBrowserSessions()?,
            String::new(),
            None,
        ))
    }
}

impl WebBrowserSessionHost {
    /// Updates one session with a browser-navigable URL.
    fn navigate(&self, command: BrowserSessionCommand) -> HostResult<BrowserSessionCommandResult> {
        let sessionId = commandSessionId(&command)?;
        let url = command
            .url
            .ok_or_else(|| HostError::new("browser navigation requires a URL"))?;
        let session = BROWSER_SESSIONS.with(|sessions| -> HostResult<BrowserSessionInfo> {
            let mut sessions = sessions.borrow_mut();
            let session = browserSession(&mut sessions, &sessionId)?;
            session.info.currentUrl = url.clone();
            session.info.title = url;
            session.info.active = true;
            session.info.canGoBack = true;
            session.info.isLoading = false;
            session.info.progress = 100;
            session.resultJson = "{}".to_string();
            Ok(session.info.clone())
        })?;
        Ok(browserResult(
            Some(session),
            self.listBrowserSessions()?,
            "{}".to_string(),
            None,
        ))
    }

    /// Evaluates JavaScript in the current browser global scope.
    fn evaluate(&self, command: BrowserSessionCommand) -> HostResult<BrowserSessionCommandResult> {
        let sessionId = commandSessionId(&command)?;
        let script = command
            .script
            .ok_or_else(|| HostError::new("browser script command requires script content"))?;
        let resultJson = js_sys::eval(&script)
            .map_err(jsError)
            .and_then(jsValueJson)?;
        let session = BROWSER_SESSIONS.with(|sessions| -> HostResult<BrowserSessionInfo> {
            let mut sessions = sessions.borrow_mut();
            let session = browserSession(&mut sessions, &sessionId)?;
            session.resultJson = resultJson.clone();
            Ok(session.info.clone())
        })?;
        Ok(browserResult(
            Some(session),
            self.listBrowserSessions()?,
            resultJson,
            None,
        ))
    }
}

/// Allocates one unique simulated browser session identifier.
#[allow(non_snake_case)]
fn nextBrowserSessionId() -> String {
    NEXT_BROWSER_SESSION_ID.with(|next| {
        let value = next.get().saturating_add(1);
        next.set(value);
        format!("web-browser-{value}")
    })
}

/// Returns one mutable simulated browser session.
#[allow(non_snake_case)]
fn browserSession<'a>(
    sessions: &'a mut BTreeMap<String, WebBrowserSession>,
    sessionId: &str,
) -> HostResult<&'a mut WebBrowserSession> {
    sessions
        .get_mut(sessionId)
        .ok_or_else(|| HostError::new(format!("browser session not found: {sessionId}")))
}

/// Extracts the required session identifier from one browser command.
#[allow(non_snake_case)]
fn commandSessionId(command: &BrowserSessionCommand) -> HostResult<String> {
    command.sessionId.clone().ok_or_else(|| {
        HostError::new(format!(
            "browser command requires sessionId: {}",
            command.action
        ))
    })
}

/// Builds one normalized browser simulator command result.
#[allow(non_snake_case)]
fn browserResult(
    session: Option<BrowserSessionInfo>,
    sessions: Vec<BrowserSessionInfo>,
    resultJson: String,
    error: Option<String>,
) -> BrowserSessionCommandResult {
    BrowserSessionCommandResult {
        success: error.is_none(),
        session,
        sessions,
        resultJson,
        error,
    }
}

/// Converts JavaScript evaluation failures into host errors.
#[allow(non_snake_case)]
fn jsError(error: JsValue) -> HostError {
    HostError::new(format!("browser script evaluation failed: {error:?}"))
}

/// Serializes one JavaScript evaluation result for the host contract.
#[allow(non_snake_case)]
fn jsValueJson(value: JsValue) -> HostResult<String> {
    JSON::stringify(&value)
        .map_err(jsError)?
        .as_string()
        .ok_or_else(|| HostError::new("browser script result cannot be serialized"))
}
