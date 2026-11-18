//! Incremental POST responses for Host-provided Streamable HTTP MCP transports.
use super::*;
use operit_host_api::HttpResponseHead;

const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_EVENT_RESUMPTIONS: usize = 3;

pub(super) enum ResponseEvent {
    Head(HttpResponseHead),
    Bytes(Vec<u8>),
    Closed(Result<(), String>),
}

/// An asynchronous response owned by its Host, cancelled on every exit path.
pub(super) struct ResponseStream {
    host: Arc<dyn HttpHost>,
    id: String,
    events: tokio::sync::mpsc::UnboundedReceiver<ResponseEvent>,
}

impl ResponseStream {
    /// Opens a Host-managed response stream that delivers headers before body bytes.
    pub(super) fn open(host: Arc<dyn HttpHost>, request: HttpRequestData) -> Result<Self, String> {
        let id = format!("mcp-http-{}", uuid::Uuid::new_v4());
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        let headSender = sender.clone();
        let chunkSender = sender.clone();
        host.openHttpResponseStream(
            id.clone(),
            request,
            Arc::new(move |head| {
                let _ = headSender.send(ResponseEvent::Head(head));
            }),
            Arc::new(move |bytes| {
                let _ = chunkSender.send(ResponseEvent::Bytes(bytes));
            }),
            Arc::new(move |result| {
                let _ = sender.send(ResponseEvent::Closed(result));
            }),
        )
        .map_err(|error| error.to_string())?;
        Ok(Self { host, id, events })
    }

    /// Receives one response event within the caller's deadline.
    pub(super) async fn next(
        &mut self,
        deadline: &StartupDeadline,
    ) -> Result<ResponseEvent, String> {
        let remaining = deadline
            .remainingMs()
            .map_err(|error| format!("Remote MCP HTTP request timed out: {error}"))?;
        tokio::select! {
            biased;
            event = self.events.recv() => {
                // A ready event must not let an expired deadline bypass the budget.
                deadline.remainingMs()?;
                Ok(event.unwrap_or_else(|| ResponseEvent::Closed(Err("Remote MCP HTTP stream disconnected".into()))))
            }
            result = deadline.scheduler.waitForHostRuntimeDelay(remaining) => {
                result.map_err(|error| error.to_string())?;
                Err("Remote MCP HTTP request timed out".to_string())
            }
        }
    }
}

impl Drop for ResponseStream {
    /// Releases the Host stream when its consumer exits.
    fn drop(&mut self) {
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}

/// Sends a request to the Streamable HTTP endpoint.
pub(super) async fn sendJsonRpc(
    session: &mut RemoteMcpSession,
    payload: Value,
    expectedId: Option<u64>,
    timeoutMs: u64,
) -> Result<Option<Value>, String> {
    let endpoint = session.endpoint.clone();
    sendJsonRpcTo(session, payload, expectedId, timeoutMs, &endpoint).await
}

/// Starts the optional GET stream without delaying POST-based discovery.
fn connectServerEvents(session: &mut RemoteMcpSession, deadline: &StartupDeadline) {
    session.pendingServerEvents = Some(openServerEvents(session, None, deadline));
}

/// Opens a GET stream, attaching the server's event cursor when resuming.
fn openServerEvents(
    session: &RemoteMcpSession,
    lastEventId: Option<&str>,
    deadline: &StartupDeadline,
) -> Result<ResponseStream, String> {
    let mut headers = buildRemoteHeaders(session, false)?;
    if let Some(lastEventId) = lastEventId {
        headers.retain(|(name, _)| name != "last-event-id");
        headers.push(("last-event-id".to_string(), lastEventId.to_string()));
    }
    let mut request = remoteHttpRequest(
        "GET",
        &session.endpoint,
        headers,
        Vec::new(),
        deadline.remainingMs()?,
    );
    request.readTimeoutSeconds = 0;
    ResponseStream::open(session.httpHost.clone(), request)
}

/// Validates the optional event channel when a JSON-RPC result actually needs it.
async fn acceptServerEvents(
    mut stream: ResponseStream,
    deadline: &StartupDeadline,
) -> Result<RemoteSseReader, String> {
    let head = match stream.next(deadline).await? {
        ResponseEvent::Head(head) => head,
        ResponseEvent::Closed(Err(error)) => return Err(error),
        _ => return Err("Remote MCP GET stream did not send response headers".to_string()),
    };
    if !isSuccess(head.statusCode) {
        return Err(format!(
            "Remote MCP GET stream failed with status {}",
            head.statusCode
        ));
    }
    let content_type = head
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.split(';').next().unwrap_or_default().trim());
    if !content_type.is_some_and(|value| value.eq_ignore_ascii_case("text/event-stream")) {
        return Err("Remote MCP GET stream must return text/event-stream".to_string());
    }
    Ok(RemoteSseReader::new(stream))
}

/// Reads a result from an event stream and resumes a disconnected stream by event ID.
async fn readResumableResponse(
    session: &RemoteMcpSession,
    reader: &mut RemoteSseReader,
    expectedId: u64,
    deadline: &StartupDeadline,
) -> Result<Value, String> {
    let mut resumptions = 0;
    loop {
        match readRemoteSseResponse(reader, expectedId, deadline).await? {
            RemoteSseResult::Response(response) => return Ok(response),
            RemoteSseResult::Closed(error) => {
                let Some(cursor) = reader.lastEventId.as_deref() else {
                    return Err(format!("Remote MCP SSE stream ended before response {expectedId}: {error}"));
                };
                if resumptions == MAX_EVENT_RESUMPTIONS {
                    return Err(format!("Remote MCP SSE resumption limit reached: {error}"));
                }
                let stream = openServerEvents(session, Some(cursor), deadline)?;
                let mut resumed = acceptServerEvents(stream, deadline).await?;
                resumed.lastEventId = Some(cursor.to_string());
                *reader = resumed;
                resumptions += 1;
            }
        }
    }
}

/// Reads a 202 result from the session's GET event channel without replaying its POST.
async fn readAcceptedResponse(
    session: &mut RemoteMcpSession,
    expectedId: u64,
    deadline: &StartupDeadline,
) -> Result<Value, String> {
    let mut reader = match session.sseReader.take() {
        Some(reader) => reader,
        None => {
            let stream = session.pendingServerEvents.take()
                .ok_or("Remote MCP returned HTTP 202 without an open event channel")??;
            acceptServerEvents(stream, deadline).await?
        }
    };
    let result = readResumableResponse(session, &mut reader, expectedId, deadline).await;
    session.sseReader = Some(reader);
    result
}

/// Legacy SSE POSTs only acknowledge delivery; their responses arrive on the GET stream.
pub(super) async fn sendJsonRpcTo(
    session: &mut RemoteMcpSession,
    payload: Value,
    expectedId: Option<u64>,
    timeoutMs: u64,
    endpoint: &str,
) -> Result<Option<Value>, String> {
    let deadline = StartupDeadline::new(session.scheduler.clone(), timeoutMs)?;
    let mut request = remoteHttpRequest(
        "POST",
        endpoint,
        buildRemoteHeaders(session, true)?,
        serde_json::to_vec(&payload).map_err(|error| error.to_string())?,
        timeoutMs,
    );
    request.readTimeoutSeconds = 0;
    let mut stream = ResponseStream::open(session.httpHost.clone(), request)?;
    let head = match stream.next(&deadline).await? {
        ResponseEvent::Head(head) => head,
        ResponseEvent::Closed(Err(error)) => return Err(error),
        _ => {
            return Err(
                "Remote MCP HTTP stream closed or sent data before response headers".to_string(),
            )
        }
    };
    if !isSuccess(head.statusCode) {
        // Bound diagnostics by the same deadline; never wait for an endless error body.
        let detail = match stream.next(&deadline).await {
            Ok(ResponseEvent::Bytes(bytes)) => {
                let mut preview = bytes[..bytes.len().min(4096)].to_vec();
                while preview.len() < 4096 {
                    match stream.events.try_recv() {
                        Ok(ResponseEvent::Bytes(bytes)) => {
                            preview
                                .extend_from_slice(&bytes[..bytes.len().min(4096 - preview.len())]);
                        }
                        _ => break,
                    }
                }
                String::from_utf8_lossy(&preview).trim().to_string()
            }
            _ => head.statusMessage.clone(),
        };
        return Err(format!(
            "Remote MCP HTTP request failed with status {}: {}",
            head.statusCode, detail
        ));
    }
    rememberRemoteSessionId(session, &head.headers)?;
    // Notifications are acknowledged by HTTP status. No body or EOF is required.
    let Some(expectedId) = expectedId else {
        if session.transport == RemoteTransport::StreamableHttp
            && head.statusCode == 202
            && payload.get("method").and_then(Value::as_str) == Some("notifications/initialized")
        {
            connectServerEvents(session, &deadline);
        }
        return Ok(None);
    };
    if head.statusCode == 202 {
        drop(stream);
        return readAcceptedResponse(session, expectedId, &deadline)
            .await
            .map(Some);
    }
    if head.statusCode == 204 {
        return Err(format!(
            "Remote MCP HTTP response {} did not contain JSON-RPC response {expectedId}",
            head.statusCode
        ));
    }
    let contentType = head
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.to_ascii_lowercase())
        .unwrap_or_default();
    let mediaType = contentType.split(';').next().unwrap_or_default().trim();
    match mediaType {
        "text/event-stream" => {
            return readResumableResponse(session, &mut RemoteSseReader::new(stream), expectedId, &deadline)
                .await
                .map(Some);
        }
        "application/json" | "" => {}
        other => return Err(format!("Unsupported remote MCP HTTP content type: {other}")),
    }
    let mut decoder = ResponseDecoder::new(expectedId);
    loop {
        match stream.next(&deadline).await? {
            ResponseEvent::Bytes(bytes) => {
                if let Some(response) = decoder.push(&bytes)? {
                    deadline
                        .remainingMs()
                        .map_err(|error| format!("Remote MCP HTTP request timed out: {error}"))?;
                    return Ok(Some(response));
                }
            }
            ResponseEvent::Closed(Err(error)) => {
                return Err(format!("Remote MCP HTTP stream failed: {error}"))
            }
            ResponseEvent::Closed(Ok(())) => return decoder.finish().map(Some),
            ResponseEvent::Head(_) => {
                return Err("Remote MCP HTTP stream sent duplicate response headers".to_string())
            }
        }
    }
}

struct ResponseDecoder {
    expectedId: u64,
    buffer: Vec<u8>,
}

impl ResponseDecoder {
    /// Creates a buffered decoder for a JSON response to one RPC request.
    fn new(expectedId: u64) -> Self {
        Self {
            expectedId,
            buffer: Vec::new(),
        }
    }

    /// Returns a complete JSON-RPC response as soon as enough bytes arrive.
    fn push(&mut self, bytes: &[u8]) -> Result<Option<Value>, String> {
        if self.buffer.len().saturating_add(bytes.len()) > MAX_RESPONSE_BYTES {
            return Err("Remote MCP HTTP response exceeds 16 MiB limit".to_string());
        }
        self.buffer.extend_from_slice(bytes);
        match serde_json::from_slice::<Value>(&self.buffer) {
            Ok(response) => {
                if response.get("id").and_then(Value::as_u64) != Some(self.expectedId)
                    || (response.get("result").is_none() && response.get("error").is_none())
                {
                    return Err(format!("Remote MCP JSON response does not match request {}", self.expectedId));
                }
                Ok(Some(response))
            }
            Err(error) if error.is_eof() => Ok(None),
            Err(error) => Err(format!("Invalid remote MCP JSON response: {error}")),
        }
    }

    /// Reports an incomplete JSON response after the server closes its body.
    fn finish(&self) -> Result<Value, String> {
        Err(format!(
            "Remote MCP HTTP stream ended before response {}",
            self.expectedId
        ))
    }
}

#[cfg(test)]
#[path = "MCPStreamableHttpTests.rs"]
mod tests;
