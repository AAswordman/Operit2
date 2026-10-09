//! Deterministic tests for live legacy SSE transport, independent of public servers.
use super::*;
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpImageDelivery, HttpStreamChunkCallback, HttpStreamClosedCallback,
    HttpStreamHost, HttpStreamOpenedCallback, HttpStreamResponseCallback, HttpResponseHead,
};
use std::time::{Duration, Instant};

struct StreamingHost {
    initial: Vec<u8>,
    closeOnOpen: bool,
    postDelay: Duration,
    chunk: Mutex<Option<HttpStreamChunkCallback>>,
    closed: Mutex<Option<HttpStreamClosedCallback>>,
    requests: Mutex<Vec<HttpRequestData>>,
    cancelled: Mutex<Vec<String>>,
    getStreamId: Mutex<Option<String>>,
}

impl StreamingHost {
    fn new(initial: &str) -> Self {
        Self {
            initial: initial.as_bytes().to_vec(),
            closeOnOpen: false,
            postDelay: Duration::ZERO,
            chunk: Mutex::new(None),
            closed: Mutex::new(None),
            requests: Mutex::new(Vec::new()),
            cancelled: Mutex::new(Vec::new()),
            getStreamId: Mutex::new(None),
        }
    }

    fn emit(&self, response: Value) {
        let data = format!("event: message\r\ndata: {response}\r\n\r\n");
        // Split even inside UTF-8 and CRLF to exercise incremental decoding.
        let chunk = self.chunk.lock().unwrap().clone().unwrap();
        for byte in data.as_bytes() {
            chunk(vec![*byte]);
        }
    }
}

impl HttpStreamHost for StreamingHost {
    fn openHttpByteStream(&self, _: String, _: HttpRequestData, _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback, _: HttpStreamClosedCallback) -> HostResult<()> {
        panic!("MCP must request HTTP response metadata")
    }
    fn openHttpResponseStream(&self, streamId: String, request: HttpRequestData,
        head: HttpStreamResponseCallback, onChunk: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback) -> HostResult<()> {
        if request.method == "POST" {
            let response = self.executeHttpRequest(request)?;
            head(HttpResponseHead { finalUrl: response.finalUrl, statusCode: response.statusCode,
                statusMessage: response.statusMessage, headers: response.headers });
            if !response.body.is_empty() { onChunk(response.body); }
            onClosed(Ok(()));
            return Ok(());
        }
        head(HttpResponseHead { finalUrl: request.url.clone(), statusCode: 200,
            statusMessage: "OK".into(), headers: vec![("Content-Type".into(), "text/event-stream".into())] });
        self.requests.lock().unwrap().push(request);
        *self.getStreamId.lock().unwrap() = Some(streamId);
        *self.chunk.lock().unwrap() = Some(onChunk.clone());
        *self.closed.lock().unwrap() = Some(onClosed.clone());
        for byte in &self.initial { onChunk(vec![*byte]); }
        if self.closeOnOpen { onClosed(Err("scripted connection loss".to_string())); }
        Ok(())
    }

    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        if self.getStreamId.lock().unwrap().as_deref() == Some(id) {
            self.cancelled.lock().unwrap().push(id.to_string());
            self.chunk.lock().unwrap().take();
            self.closed.lock().unwrap().take();
        }
        Ok(())
    }
}

impl HttpHost for StreamingHost {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }

    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        // A buffered GET would block forever with an actual SSE server.
        assert_eq!(
            request.method, "POST",
            "legacy SSE GET must use a live byte stream"
        );
        assert_eq!(request.url, "https://test.invalid/message?sessionId=one");
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        self.requests.lock().unwrap().push(request.clone());
        std::thread::sleep(self.postDelay);
        let id = payload.get("id");
        if let Some(id) = id {
            let response = match payload["method"].as_str().unwrap() {
                "initialize" => json!({"jsonrpc": "2.0", "id": id,
                    "result": {"protocolVersion": "2024-11-05", "capabilities": {},
                        "serverInfo": {"name": "fixture", "version": "1"}}}),
                "tools/list" => json!({"jsonrpc": "2.0", "id": id,
                    "result": {"tools": [{"name": "echo", "inputSchema": {"type": "object"}}]}}),
                "tools/call" if payload["params"]["name"] == "echo" => {
                    json!({"jsonrpc": "2.0", "id": id,
                    "result": {"content": [{"type": "text", "text": "实时读取正常"}]}})
                }
                _ => {
                    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "unknown tool"}})
                }
            };
            // Interleave server notifications with the matching response.
            self.emit(json!({"jsonrpc": "2.0", "method": "notifications/message", "params": {}}));
            self.emit(response);
        }
        Ok(HttpResponseData {
            finalUrl: request.url,
            statusCode: 202,
            statusMessage: "Accepted".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        })
    }

    fn downloadFiles(
        &self,
        _request: HttpDownloadRequest,
        _control: HttpDownloadControl,
        _progress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("not used in MCP tests"))
    }
}

fn service() -> RegisteredService {
    RegisteredService {
        name: "fixture".to_string(),
        serviceType: "remote".to_string(),
        command: String::new(),
        args: Vec::new(),
        cwd: None,
        endpoint: Some("https://test.invalid/sse".to_string()),
        connectionType: Some("sse".to_string()),
        bearerToken: None,
        headers: BTreeMap::new(),
        description: String::new(),
        env: BTreeMap::new(),
        startupGate: Arc::new(tokio::sync::Mutex::new(())),
    }
}

const ENDPOINT: &str = ": heartbeat\r\n\r\nevent: endpoint\r\ndata: /message?sessionId=one\r\n\r\n";

#[tokio::test]
async fn liveSseHandshakeAndRepeatedCallsReadFutureChunks() {
    let host = Arc::new(StreamingHost::new(ENDPOINT));
    let mut active = startRemoteServiceSession(
        host.clone(),
        &service(),
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    ).await
    .unwrap();
    assert!(active.ready);
    assert_eq!(active.tools[0]["name"], "echo");
    for _ in 0..2 {
        let result = callRemoteMcpTool(&mut active, "echo", json!({}), 1000).await.unwrap();
        assert_eq!(result["content"][0]["text"], "实时读取正常");
    }
    let before = host.requests.lock().unwrap().len();
    assert!(callRemoteMcpTool(&mut active, "unknown", json!({}), 1000).await
        .unwrap_err()
        .contains("unknown tool"));
    assert_eq!(
        host.requests.lock().unwrap().len(),
        before + 1,
        "tool calls must not be replayed"
    );
    assert!(host.cancelled.lock().unwrap().is_empty());
    drop(active);
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn missingEndpointHonorsStartupDeadlineAndCancelsStream() {
    let host = Arc::new(StreamingHost::new(": heartbeat\n\n"));
    let started = Instant::now();
    let error = startRemoteServiceSession(
        host.clone(),
        &service(),
        &StartupDeadline::new(testScheduler(), 30).unwrap(),
    ).await
    .err()
    .unwrap();
    assert!(error.contains("timed out"), "{error}");
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    assert_eq!(host.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn closedStreamFailsImmediatelyInsteadOfSpinningUntilDeadline() {
    let mut scripted = StreamingHost::new("");
    scripted.closeOnOpen = true;
    let host = Arc::new(scripted);
    let started = Instant::now();
    let error = startRemoteServiceSession(
        host.clone(),
        &service(),
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    ).await
    .err()
    .unwrap();
    assert!(error.contains("scripted connection loss"), "{error}");
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn crossOriginEndpointIsRejectedAndStreamCancelled() {
    let host = Arc::new(StreamingHost::new(
        "event: endpoint\ndata: https://other.invalid/message\n\n",
    ));
    let mut registered = service();
    registered.bearerToken = Some("secret-for-same-origin-only".to_string());
    let error = startRemoteServiceSession(
        host.clone(),
        &registered,
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    ).await
    .err()
    .unwrap();
    assert!(error.contains("origin does not match"), "{error}");
    assert_eq!(
        host.requests.lock().unwrap().len(),
        1,
        "must not POST secrets to another origin"
    );
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn ssePostAndResponseWaitShareOneBudget() {
    let mut scripted = StreamingHost::new(ENDPOINT);
    scripted.postDelay = Duration::from_millis(40);
    let host = Arc::new(scripted);
    let mut active = startRemoteServiceSession(
        host.clone(),
        &service(),
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    ).await
    .unwrap();
    let error = callRemoteMcpTool(&mut active, "echo", json!({}), 10).await.unwrap_err();
    assert!(error.contains("timed out"), "{error}");
    drop(active);
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn readerReportsCleanClosureAndDisconnectWithoutBusyLoop() {
    use super::streamable_http::{ResponseStream, ResponseEvent};
    for closed in [true, false] {
        let host = Arc::new(StreamingHost::new(""));
        let stream = ResponseStream::open(host.clone(), remoteHttpRequest("GET", "https://test.invalid/sse", Vec::new(), Vec::new(), 1000)).unwrap();
        let mut reader = RemoteSseReader::new(stream);
        let deadline = StartupDeadline::new(testScheduler(), 1000).unwrap();
        assert!(matches!(reader.stream.next(&deadline).await.unwrap(), ResponseEvent::Head(_)));
        if closed {
            host.closed.lock().unwrap().as_ref().unwrap()(Ok(()));
        } else {
            host.chunk.lock().unwrap().take();
            host.closed.lock().unwrap().take();
        }
        assert!(reader.next(&deadline).await.unwrap_err().contains(if closed { "closed" } else { "disconnected" }));
    }
}
