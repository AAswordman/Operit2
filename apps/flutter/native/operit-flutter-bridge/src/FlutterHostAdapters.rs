use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerBrowserAutomation, requestOwnerBrowserSession,
    requestOwnerComposeWebViewController, requestOwnerWebVisit,
    RuntimeHostInteractionBrowserAutomationPayload, RuntimeHostInteractionBrowserSessionPayload,
    RuntimeHostInteractionComposeWebViewControllerPayload, RuntimeHostInteractionWebVisitHeader,
    RuntimeHostInteractionWebVisitPayload,
};

use crate::current_time_millis_u64;

/// Forwards browser automation work from the runtime to the Flutter owner.
#[derive(Clone)]
pub(crate) struct FlutterBrowserAutomationBridge {}

impl FlutterBrowserAutomationBridge {
    /// Creates the browser automation owner bridge.
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl operit_host_api::BrowserAutomationHost for FlutterBrowserAutomationBridge {
    /// Executes one browser automation request through the Flutter owner.
    fn executeBrowserTool(
        &self,
        request: operit_host_api::BrowserAutomationRequest,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserAutomationResponse> {
        let requestId = request.requestId.clone();
        let pending = RuntimeHostInteractionBrowserAutomationPayload {
            requestId: request.requestId,
            toolName: request.toolName,
            parametersJson: request.parametersJson,
            requestedAtMillis: current_time_millis_u64(),
        };
        let response = requestOwnerBrowserAutomation(pending, Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        if response.requestId != requestId {
            return Err(operit_host_api::HostError::new(format!(
                "browser automation response requestId mismatch: {} != {requestId}",
                response.requestId
            )));
        }
        if response.success {
            return Ok(operit_host_api::BrowserAutomationResponse {
                output: response.result,
            });
        }
        let Some(error) = response.error else {
            return Err(operit_host_api::HostError::new(
                "browser automation error is missing",
            ));
        };
        Err(operit_host_api::HostError::new(error))
    }
}

/// Forwards browser session commands from the runtime to the Flutter owner.
#[derive(Clone)]
pub(crate) struct FlutterBrowserSessionBridge {}

impl FlutterBrowserSessionBridge {
    /// Creates a browser session bridge that delegates to the owner app.
    pub(crate) fn new() -> Self {
        Self {}
    }

    /// Sends one browser session command to the owner app.
    fn requestCommand(
        &self,
        command: operit_host_api::BrowserSessionCommand,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionCommandResult> {
        let commandJson = serde_json::to_string(&command).map_err(|error| {
            operit_host_api::HostError::new(format!(
                "browser session command encode failed: {error}"
            ))
        })?;
        let response = requestOwnerBrowserSession(
            RuntimeHostInteractionBrowserSessionPayload { commandJson },
            Duration::from_secs(60),
        )
        .map_err(operit_host_api::HostError::new)?;
        serde_json::from_str(&response.resultJson).map_err(|error| {
            operit_host_api::HostError::new(format!(
                "browser session response decode failed: {error}"
            ))
        })
    }

    /// Builds a browser session command envelope.
    fn command(action: &str) -> operit_host_api::BrowserSessionCommand {
        operit_host_api::BrowserSessionCommand {
            action: action.to_string(),
            sessionId: None,
            url: None,
            script: None,
            payloadJson: String::new(),
            userAgent: None,
            headers: BTreeMap::new(),
        }
    }

    /// Requires the command result to include a browser session.
    fn requireSession(
        result: operit_host_api::BrowserSessionCommandResult,
        operation: &str,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionInfo> {
        result.session.ok_or_else(|| {
            operit_host_api::HostError::new(format!(
                "browser session {operation} result is missing session"
            ))
        })
    }
}

impl operit_host_api::BrowserSessionHost for FlutterBrowserSessionBridge {
    /// Lists interactive browser sessions owned by the Flutter app.
    fn listBrowserSessions(
        &self,
    ) -> operit_host_api::HostResult<Vec<operit_host_api::BrowserSessionInfo>> {
        let result = self.requestCommand(Self::command("list"))?;
        Ok(result.sessions)
    }

    /// Creates an interactive browser session in the Flutter app.
    fn createBrowserSession(
        &self,
        initialUrl: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionInfo> {
        let mut command = Self::command("create");
        command.url = Some(initialUrl.to_string());
        command.userAgent = userAgent.map(str::to_string);
        command.headers = headers;
        Self::requireSession(self.requestCommand(command)?, "create")
    }

    /// Updates a browser session owned by the Flutter app.
    fn updateBrowserSession(
        &self,
        sessionId: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionInfo> {
        let mut command = Self::command("update");
        command.sessionId = Some(sessionId.to_string());
        command.userAgent = userAgent.map(str::to_string);
        command.headers = headers;
        Self::requireSession(self.requestCommand(command)?, "update")
    }

    /// Submits a semantic browser command to the Flutter app.
    fn submitBrowserCommand(
        &self,
        command: operit_host_api::BrowserSessionCommand,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionCommandResult> {
        self.requestCommand(command)
    }

    /// Reads a browser session snapshot from the Flutter app.
    fn getBrowserSessionSnapshot(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionSnapshot> {
        let mut command = Self::command("snapshot");
        command.sessionId = Some(sessionId.to_string());
        let result = self.requestCommand(command)?;
        let session = Self::requireSession(result.clone(), "snapshot")?;
        Ok(operit_host_api::BrowserSessionSnapshot {
            session,
            resultJson: result.resultJson,
        })
    }

    /// Closes a browser session owned by the Flutter app.
    fn closeBrowserSession(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionCommandResult> {
        let mut command = Self::command("close");
        command.sessionId = Some(sessionId.to_string());
        self.requestCommand(command)
    }
}

/// Forwards web visits from the runtime to the Flutter owner.
#[derive(Clone)]
pub(crate) struct FlutterWebVisitBridge {}

impl FlutterWebVisitBridge {
    /// Creates the web visit owner bridge.
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl operit_host_api::WebVisitHost for FlutterWebVisitBridge {
    /// Visits one web page through the Flutter owner.
    fn visitWeb(
        &self,
        request: operit_host_api::WebVisitRequest,
    ) -> operit_host_api::HostResult<operit_host_api::WebVisitResult> {
        static NEXT_WEB_VISIT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
        let requestId = format!(
            "web-visit-{}-{}",
            current_time_millis_u64(),
            NEXT_WEB_VISIT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
        );
        let pending = RuntimeHostInteractionWebVisitPayload {
            requestId: requestId.clone(),
            url: request.url,
            headers: request
                .headers
                .into_iter()
                .map(|(name, value)| RuntimeHostInteractionWebVisitHeader { name, value })
                .collect(),
            userAgent: request.userAgent,
            includeImageLinks: request.includeImageLinks,
            requestedAtMillis: current_time_millis_u64(),
        };
        let response = requestOwnerWebVisit(pending, Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        if response.requestId != requestId {
            return Err(operit_host_api::HostError::new(format!(
                "web visit response requestId mismatch: {} != {requestId}",
                response.requestId
            )));
        }
        if response.success {
            let Some(result) = response.result else {
                return Err(operit_host_api::HostError::new(
                    "web visit result is missing",
                ));
            };
            return Ok(operit_host_api::WebVisitResult {
                url: result.url,
                title: result.title,
                content: result.content,
                metadata: result
                    .metadata
                    .into_iter()
                    .map(|entry| (entry.name, entry.value))
                    .collect(),
                links: result
                    .links
                    .into_iter()
                    .map(|link| operit_host_api::WebVisitLinkData {
                        url: link.url,
                        text: link.text,
                    })
                    .collect(),
                imageLinks: result.imageLinks,
            });
        }
        let Some(error) = response.error else {
            return Err(operit_host_api::HostError::new(
                "web visit error is missing",
            ));
        };
        Err(operit_host_api::HostError::new(error))
    }
}

/// Forwards Compose DSL view commands from the runtime to the Flutter owner.
#[derive(Clone)]
pub(crate) struct FlutterComposeDslWebViewBridge {}

impl FlutterComposeDslWebViewBridge {
    /// Creates the Compose DSL view owner bridge.
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl operit_host_api::ComposeDslWebViewHost for FlutterComposeDslWebViewBridge {
    /// Handles one Compose DSL controller command through the Flutter owner.
    fn handleControllerCommand(&self, payloadJson: &str) -> operit_host_api::HostResult<String> {
        let response = requestOwnerComposeWebViewController(
            RuntimeHostInteractionComposeWebViewControllerPayload {
                commandJson: payloadJson.to_string(),
            },
            Duration::from_secs(60),
        )
        .map_err(operit_host_api::HostError::new)?;
        Ok(response.result)
    }
}
