//! Incremental POST responses for Host-provided Streamable HTTP MCP transports.
use super::*;
use operit_host_api::HttpResponseHead;

const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

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
    pub(super) fn open(host: Arc<dyn HttpHost>, request: HttpRequestData) -> Result<Self, String> {
        let id = format!("mcp-http-{}", uuid::Uuid::new_v4());
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        let headSender = sender.clone();
        let chunkSender = sender.clone();
        host.openHttpResponseStream(
            id.clone(), request,
            Arc::new(move |head| { let _ = headSender.send(ResponseEvent::Head(head)); }),
            Arc::new(move |bytes| { let _ = chunkSender.send(ResponseEvent::Bytes(bytes)); }),
            Arc::new(move |result| { let _ = sender.send(ResponseEvent::Closed(result)); }),
        ).map_err(|error| error.to_string())?;
        Ok(Self { host, id, events })
    }

    pub(super) async fn next(&mut self, deadline: &StartupDeadline) -> Result<ResponseEvent, String> {
        let remaining = deadline.remainingMs()
            .map_err(|error| format!("Remote MCP HTTP request timed out: {error}"))?;
        tokio::select! {
            biased;
            event = self.events.recv() => {
                // A ready event must not let an expired deadline bypass the budget.
                deadline.remainingMs()?;
                event.ok_or_else(|| "Remote MCP HTTP stream disconnected".to_string())
            }
            result = deadline.scheduler.waitForHostRuntimeDelay(remaining) => {
                result.map_err(|error| error.to_string())?;
                Err("Remote MCP HTTP request timed out".to_string())
            }
        }
    }
}

impl Drop for ResponseStream {
    fn drop(&mut self) { let _ = self.host.closeHttpByteStream(&self.id); }
}

pub(super) async fn sendJsonRpc(
    session: &mut RemoteMcpSession, payload: Value, expectedId: Option<u64>, timeoutMs: u64,
) -> Result<Option<Value>, String> {
    let endpoint = session.endpoint.clone();
    sendJsonRpcTo(session, payload, expectedId, timeoutMs, &endpoint).await
}

/// Legacy SSE POSTs only acknowledge delivery; their responses arrive on the GET stream.
pub(super) async fn sendJsonRpcTo(
    session: &mut RemoteMcpSession, payload: Value, expectedId: Option<u64>, timeoutMs: u64,
    endpoint: &str,
) -> Result<Option<Value>, String> {
    let deadline = StartupDeadline::new(session.scheduler.clone(), timeoutMs)?;
    let mut request = remoteHttpRequest("POST", endpoint, buildRemoteHeaders(session, true)?,
        serde_json::to_vec(&payload).map_err(|error| error.to_string())?, timeoutMs);
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
        return Ok(None);
    };
    if head.statusCode == 202 || head.statusCode == 204 {
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
    let sse = match mediaType {
        "text/event-stream" => true,
        "application/json" | "" => false,
        other => return Err(format!("Unsupported remote MCP HTTP content type: {other}")),
    };
    let mut decoder = ResponseDecoder::new(sse, expectedId);
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
    sse: bool,
    expectedId: u64,
    buffer: Vec<u8>,
    eventData: String,
    eventName: String,
    skipLf: bool,
}

impl ResponseDecoder {
    fn new(sse: bool, expectedId: u64) -> Self {
        Self {
            sse,
            expectedId,
            buffer: Vec::new(),
            eventData: String::new(),
            eventName: String::new(),
            skipLf: false,
        }
    }

    fn push(&mut self, bytes: &[u8]) -> Result<Option<Value>, String> {
        let bytes = if self.skipLf && !bytes.is_empty() {
            self.skipLf = false;
            bytes.strip_prefix(b"\n").unwrap_or(bytes)
        } else {
            bytes
        };
        if self
            .buffer
            .len()
            .saturating_add(self.eventData.len())
            .saturating_add(bytes.len())
            > MAX_RESPONSE_BYTES
        {
            return Err("Remote MCP HTTP response exceeds 16 MiB limit".to_string());
        }
        self.buffer.extend_from_slice(bytes);
        if !self.sse {
            return match serde_json::from_slice::<Value>(&self.buffer) {
                Ok(response) => {
                    if !self.matches(&response) {
                        return Err(format!(
                            "Remote MCP JSON response does not match request {}",
                            self.expectedId
                        ));
                    }
                    Ok(Some(response))
                }
                Err(error) if error.is_eof() => Ok(None),
                Err(error) => Err(format!("Invalid remote MCP JSON response: {error}")),
            };
        }
        let mut consumed = 0;
        while let Some(relativeEnd) = self.buffer[consumed..]
            .iter()
            .position(|byte| *byte == b'\n' || *byte == b'\r')
        {
            let end = consumed + relativeEnd;
            let line = std::str::from_utf8(&self.buffer[consumed..end])
                .map_err(|error| format!("Invalid remote MCP SSE UTF-8: {error}"))?
                .trim_end_matches('\r')
                .to_string();
            consumed = end + 1;
            if self.buffer[end] == b'\r' {
                if self.buffer.get(consumed) == Some(&b'\n') {
                    consumed += 1;
                } else if consumed == self.buffer.len() {
                    self.skipLf = true;
                }
            }
            if line.is_empty() {
                let data = std::mem::take(&mut self.eventData);
                let event = std::mem::take(&mut self.eventName);
                if !data.is_empty()
                    && data.trim() != "[DONE]"
                    && (event.is_empty() || event == "message")
                {
                    let response: Value = serde_json::from_str(&data)
                        .map_err(|error| format!("Invalid remote MCP SSE response: {error}"))?;
                    if self.matches(&response) {
                        return Ok(Some(response));
                    }
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                if !self.eventData.is_empty() {
                    self.eventData.push('\n');
                }
                self.eventData
                    .push_str(data.strip_prefix(' ').unwrap_or(data));
            } else if let Some(event) = line.strip_prefix("event:") {
                self.eventName = event.strip_prefix(' ').unwrap_or(event).to_string();
            }
        }
        self.buffer.drain(..consumed);
        Ok(None)
    }

    fn matches(&self, response: &Value) -> bool {
        response.get("id").and_then(Value::as_u64) == Some(self.expectedId)
            && (response.get("result").is_some() || response.get("error").is_some())
    }

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
