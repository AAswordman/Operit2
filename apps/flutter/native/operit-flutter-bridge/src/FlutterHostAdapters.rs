use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerBrowserAutomation, requestOwnerBrowserSession, requestOwnerComposeFilePicker,
    requestOwnerComposeWebViewController, requestOwnerWebVisit,
    RuntimeHostInteractionBrowserAutomationPayload, RuntimeHostInteractionBrowserSessionPayload,
    RuntimeHostInteractionComposeFilePickerPayload,
    RuntimeHostInteractionComposeWebViewControllerPayload, RuntimeHostInteractionWebVisitHeader,
    RuntimeHostInteractionWebVisitPayload,
};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::{
    BrowserAutomationRequest, BrowserAutomationResponse, FileSystemHost, HostError, HostResult,
};

use crate::current_time_millis_u64;

/// Adapts browser automation operations to the Flutter owner transport.
#[derive(Clone)]
pub(crate) struct FlutterBrowserAutomationBridge {
    fileSystemHost: Arc<dyn FileSystemHost>,
}

impl FlutterBrowserAutomationBridge {
    /// Creates the owner bridge with the file-system capability of its runtime.
    pub(crate) fn new(fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self { fileSystemHost }
    }

    /// Prepares the upload-specific owner payload through the host file system.
    fn fileUpload(
        &self,
        mut request: BrowserAutomationRequest,
    ) -> HostResult<BrowserAutomationResponse> {
        let parameters: BrowserFileUploadParameters = serde_json::from_str(&request.parametersJson)
            .map_err(|error| {
                HostError::new(format!("Invalid browser upload parameters: {error}"))
            })?;
        let files = parameters
            .paths()?
            .map(|paths| {
                paths
                    .iter()
                    .map(|path| BrowserUploadFile::read(self.fileSystemHost.as_ref(), path))
                    .collect::<HostResult<Vec<_>>>()
            })
            .transpose()?;
        let payload = BrowserFileUploadOwnerParameters {
            files: serde_json::to_string(&files)
                .map_err(|error| HostError::new(error.to_string()))?,
        };
        request.parametersJson =
            serde_json::to_string(&payload).map_err(|error| HostError::new(error.to_string()))?;
        self.forward(request)
    }

    /// Sends one prepared request and validates the owner response envelope.
    fn forward(&self, request: BrowserAutomationRequest) -> HostResult<BrowserAutomationResponse> {
        let requestId = request.requestId.clone();
        let pending = RuntimeHostInteractionBrowserAutomationPayload {
            requestId: request.requestId,
            toolName: request.toolName,
            parametersJson: request.parametersJson,
            requestedAtMillis: current_time_millis_u64(),
        };
        let response = requestOwnerBrowserAutomation(pending, Duration::from_secs(60))
            .map_err(HostError::new)?;
        if response.requestId != requestId {
            return Err(HostError::new(format!(
                "browser automation response requestId mismatch: {} != {requestId}",
                response.requestId
            )));
        }
        if response.success {
            return Ok(BrowserAutomationResponse {
                output: response.result,
            });
        }
        let Some(error) = response.error else {
            return Err(HostError::new("browser automation error is missing"));
        };
        Err(HostError::new(error))
    }
}

impl operit_host_api::BrowserAutomationHost for FlutterBrowserAutomationBridge {
    /// Dispatches host operations to their owner-transport handlers.
    fn executeBrowserTool(
        &self,
        request: BrowserAutomationRequest,
    ) -> HostResult<BrowserAutomationResponse> {
        match request.toolName.as_str() {
            "browser_file_upload" => self.fileUpload(request),
            _ => self.forward(request),
        }
    }
}

/// Describes the public upload parameters received from the tool serializer.
#[derive(serde::Deserialize)]
struct BrowserFileUploadParameters {
    paths: Option<String>,
}

impl BrowserFileUploadParameters {
    /// Decodes a selection without conflating cancellation with an empty array.
    fn paths(self) -> HostResult<Option<Vec<String>>> {
        let paths = self
            .paths
            .map(|raw| {
                serde_json::from_str::<Vec<String>>(&raw).map_err(|error| {
                    HostError::new(format!("paths must be a JSON array of file paths: {error}"))
                })
            })
            .transpose()?;
        if paths
            .as_ref()
            .is_some_and(|paths| paths.iter().any(|path| path.trim().is_empty()))
        {
            return Err(HostError::new("Browser upload paths must not be empty"));
        }
        Ok(paths)
    }
}

/// Carries one upload selection in the owner's string-valued parameter protocol.
#[derive(serde::Serialize)]
struct BrowserFileUploadOwnerParameters {
    files: String,
}

/// Carries one file across the browser owner's transport boundary.
#[derive(serde::Serialize)]
struct BrowserUploadFile {
    name: String,
    base64: String,
}

impl BrowserUploadFile {
    /// Validates and reads one file exclusively through the runtime's host API.
    fn read(fileSystemHost: &dyn FileSystemHost, path: &str) -> HostResult<Self> {
        fileSystemHost.validatePath(path, "paths")?;
        let existence = fileSystemHost.fileExists(path)?;
        if !existence.exists || existence.isDirectory {
            return Err(HostError::new(format!(
                "Browser upload requires an existing file: {path}"
            )));
        }
        let name = path
            .rsplit(['/', '\\'])
            .next()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                HostError::new(format!("Browser upload path has no file name: {path}"))
            })?;
        let bytes = fileSystemHost.readFileBytes(path).map_err(|error| {
            HostError::new(format!(
                "Failed to read browser upload file {path}: {}",
                error.message
            ))
        })?;
        Ok(Self {
            name: name.to_string(),
            base64: STANDARD.encode(bytes),
        })
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
    fn handleControllerCommand(
        &self,
        payload: &serde_json::Value,
    ) -> operit_host_api::HostResult<serde_json::Value> {
        let response = requestOwnerComposeWebViewController(
            RuntimeHostInteractionComposeWebViewControllerPayload {
                commandJson: payload.to_string(),
            },
            Duration::from_secs(60),
        )
        .map_err(operit_host_api::HostError::new)?;
        serde_json::from_str(&response.result).map_err(|error| {
            operit_host_api::HostError::new(format!(
                "Invalid Compose controller owner response: {error}"
            ))
        })
    }

    /// Opens one Compose DSL file picker through the Flutter owner surface.
    fn openFilePicker(
        &self,
        request: operit_host_api::ComposeDslFilePickerRequest,
    ) -> operit_host_api::HostResult<serde_json::Value> {
        let requestJson = serde_json::to_string(&request).map_err(|error| {
            operit_host_api::HostError::new(format!(
                "Compose DSL file picker request encode failed: {error}"
            ))
        })?;
        let response = requestOwnerComposeFilePicker(
            RuntimeHostInteractionComposeFilePickerPayload { requestJson },
            Duration::from_secs(600),
        )
        .map_err(operit_host_api::HostError::new)?;
        serde_json::from_str(&response.resultJson).map_err(|error| {
            operit_host_api::HostError::new(format!(
                "Invalid Compose file-picker owner response: {error}"
            ))
        })
    }
}

#[cfg(test)]
mod browser_upload_tests {
    use super::*;

    /// Decodes the owner's public string-valued paths protocol for boundary tests.
    fn upload_paths(parameters: serde_json::Value) -> HostResult<Option<Vec<String>>> {
        let parameters: BrowserFileUploadParameters = serde_json::from_value(parameters)
            .map_err(|error| HostError::new(error.to_string()))?;
        parameters.paths()
    }

    /// Preserves cancellation and clearing as distinct upload operations.
    #[test]
    fn upload_paths_preserve_selection_intent() {
        assert_eq!(upload_paths(serde_json::json!({})).unwrap(), None);
        assert_eq!(
            upload_paths(serde_json::json!({"paths":"[]"})).unwrap(),
            Some(Vec::new())
        );
    }

    /// Rejects malformed selections instead of forwarding a fabricated upload.
    #[test]
    fn upload_paths_validate_the_host_boundary() {
        for raw in ["", "null", "{}", "42", "[1]", r#"[""]"#, r#"["  "]"#] {
            assert!(
                upload_paths(serde_json::json!({"paths":raw})).is_err(),
                "{raw}"
            );
        }
    }

    /// Keeps Unicode names, caller-defined ordering, and spaces unchanged at the host boundary.
    #[test]
    fn upload_paths_preserve_file_identity() {
        let paths = vec![
            "/tmp/中文 file.txt".to_string(),
            r"D:\uploads\second.bin".to_string(),
        ];
        let parameters = serde_json::json!({"paths":serde_json::to_string(&paths).unwrap()});
        assert_eq!(upload_paths(parameters).unwrap(), Some(paths));
    }

    /// Encodes upload intent in the exact string-valued owner parameter schema.
    #[test]
    fn owner_upload_payload_preserves_selection_intent() {
        let cancelled = BrowserFileUploadOwnerParameters {
            files: serde_json::to_string(&Option::<Vec<BrowserUploadFile>>::None).unwrap(),
        };
        let cleared = BrowserFileUploadOwnerParameters {
            files: serde_json::to_string(&Some(Vec::<BrowserUploadFile>::new())).unwrap(),
        };
        assert_eq!(
            serde_json::to_value(cancelled).unwrap(),
            serde_json::json!({"files":"null"})
        );
        assert_eq!(
            serde_json::to_value(cleared).unwrap(),
            serde_json::json!({"files":"[]"})
        );
    }
}
