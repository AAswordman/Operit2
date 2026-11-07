use super::*;
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpImageDelivery, HttpStreamChunkCallback, HttpStreamClosedCallback,
    HttpStreamHost, HttpStreamOpenedCallback, HttpStreamResponseCallback,
};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

struct ScriptedResponse {
    head: Option<HttpResponseHead>,
    chunks: Vec<Vec<u8>>,
    closed: Option<Result<(), String>>,
}

impl ScriptedResponse {
    fn response(contentType: &str, body: &str) -> Self {
        Self {
            head: Some(HttpResponseHead {
                finalUrl: "https://test.invalid/mcp".into(),
                statusCode: 200,
                statusMessage: "OK".into(),
                headers: vec![("Content-Type".into(), contentType.into())],
            }),
            chunks: body
                .as_bytes()
                .chunks(1)
                .map(|bytes| bytes.to_vec())
                .collect(),
            closed: None,
        }
    }
}

#[derive(Default)]
struct ResponseHost {
    responses: Mutex<VecDeque<ScriptedResponse>>,
    requests: Mutex<Vec<HttpRequestData>>,
    streams: Mutex<BTreeMap<String, (HttpStreamChunkCallback, HttpStreamClosedCallback)>>,
    cancelled: Mutex<Vec<String>>,
}

impl ResponseHost {
    fn with(responses: Vec<ScriptedResponse>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into()),
            ..Self::default()
        })
    }
}

impl HttpStreamHost for ResponseHost {
    fn openHttpByteStream(
        &self,
        _id: String,
        _request: HttpRequestData,
        _opened: HttpStreamOpenedCallback,
        _chunk: HttpStreamChunkCallback,
        _closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("Streamable HTTP must request response metadata")
    }

    fn openHttpResponseStream(
        &self,
        id: String,
        request: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        assert!(matches!(request.method.as_str(), "POST" | "GET"));
        self.requests.lock().unwrap().push(request);
        self.streams
            .lock()
            .unwrap()
            .insert(id, (chunk.clone(), closed.clone()));
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected additional HTTP request");
        if let Some(responseHead) = response.head {
            head(responseHead);
        }
        for bytes in response.chunks {
            chunk(bytes);
        }
        if let Some(result) = response.closed {
            closed(result);
        }
        Ok(())
    }

    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        assert!(self.streams.lock().unwrap().remove(id).is_some());
        self.cancelled.lock().unwrap().push(id.to_string());
        Ok(())
    }
}

impl HttpHost for ResponseHost {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    fn executeHttpRequest(&self, _request: HttpRequestData) -> HostResult<HttpResponseData> {
        panic!("Streamable HTTP must never fall back to buffered requests")
    }
    fn downloadFiles(
        &self,
        _request: HttpDownloadRequest,
        _control: HttpDownloadControl,
        _progress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("not used"))
    }
}

fn session(host: Arc<dyn HttpHost>) -> RemoteMcpSession {
    RemoteMcpSession {
        scheduler: testScheduler(),
        httpHost: host,
        endpoint: "https://test.invalid/mcp".into(),
        connectionType: "httpStream".into(),
        headers: BTreeMap::new(),
        sessionId: None,
        protocolVersion: "2024-11-05".into(),
        sseEndpoint: None,
        sseReader: None,
    }
}

fn request(id: u64) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/list"})
}

#[tokio::test]
async fn heldOpenSseReturnsMatchingResultWithoutEofAndCancels() {
    let wire = concat!(": heartbeat\r\n\r\n",
        "event: message\r\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\r\n\r\n",
        "data: {\"jsonrpc\":\"2.0\",\"id\":99,\"result\":{}}\r\n\r\n",
        "event: message\r\ndata: {\"jsonrpc\":\"2.0\",\"id\":7,\n",
        "data: \"result\":{\"text\":\"实时读取\"}}\r\n\r\n");
    let host = ResponseHost::with(vec![ScriptedResponse::response(
        "text/event-stream; charset=utf-8",
        wire,
    )]);
    let started = Instant::now();
    let response = sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 1000)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response["result"]["text"], "实时读取");
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    assert!(host.streams.lock().unwrap().is_empty());
}

#[tokio::test]
async fn heldOpenJsonReturnsAsSoonAsComplete() {
    let host = ResponseHost::with(vec![ScriptedResponse::response(
        "application/json",
        "{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}",
    )]);
    assert!(
        sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 1000)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn deadlinesCoverHeadersAndIncompleteBodies() {
    for response in [
        ScriptedResponse {
            head: None,
            chunks: Vec::new(),
            closed: None,
        },
        ScriptedResponse::response("text/event-stream", ": heartbeat\n\ndata: {\"id\":7"),
        ScriptedResponse::response("application/json", "{\"jsonrpc\":\"2.0\",\"id\":7"),
    ] {
        let host = ResponseHost::with(vec![response]);
        let started = Instant::now();
        let error = sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 25)
            .await
            .unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn notificationNeedsOnlyAcceptedStatusAndStillCancelsBody() {
    let mut response = ScriptedResponse::response("", "");
    response.head.as_mut().unwrap().statusCode = 202;
    let host = ResponseHost::with(vec![response]);
    assert_eq!(
        sendJsonRpc(
            &mut session(host.clone()),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            None,
            1000
        )
        .await
        .unwrap(),
        None
    );
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn failuresNeverReplayAndAlwaysCancel() {
    for (mut response, needle) in [
        (
            ScriptedResponse::response("application/json", "{\"id\":99,\"result\":{}}"),
            "does not match",
        ),
        (
            ScriptedResponse::response("application/json", "not-json"),
            "Invalid remote MCP JSON",
        ),
        (
            ScriptedResponse::response("text/event-stream", "data: not-json\n\n"),
            "Invalid remote MCP SSE",
        ),
        (
            ScriptedResponse::response("text/plain", "ok"),
            "Unsupported",
        ),
    ] {
        response.closed = Some(Ok(()));
        let host = ResponseHost::with(vec![response]);
        let error = sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 1000)
            .await
            .unwrap_err();
        assert!(error.contains(needle), "{error}");
        assert_eq!(host.requests.lock().unwrap().len(), 1);
        assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn prematureEofAndConnectionErrorsFailImmediately() {
    for (closed, needle) in [
        (Ok(()), "ended before response"),
        (Err("connection lost".into()), "connection lost"),
    ] {
        let mut response = ScriptedResponse::response("text/event-stream", ": heartbeat\n\n");
        response.closed = Some(closed);
        let host = ResponseHost::with(vec![response]);
        let started = Instant::now();
        let error = sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 1000)
            .await
            .unwrap_err();
        assert!(error.contains(needle), "{error}");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn httpErrorKeepsStatusAndBoundedDiagnosticsWithoutPoisoningSession() {
    let mut response = ScriptedResponse::response("application/json", "authentication required");
    response.head.as_mut().unwrap().statusCode = 401;
    response
        .head
        .as_mut()
        .unwrap()
        .headers
        .push(("mcp-session-id".into(), "untrusted-error-session".into()));
    let host = ResponseHost::with(vec![response]);
    let mut session = session(host.clone());
    session.sessionId = Some("valid-session".into());
    let error = sendJsonRpc(&mut session, request(7), Some(7), 1000)
        .await
        .unwrap_err();
    assert!(error.contains("status 401"), "{error}");
    assert!(error.contains("authentication"), "{error}");
    assert_eq!(session.sessionId.as_deref(), Some("valid-session"));
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn sessionAndNegotiatedVersionSurviveIncrementalHandshake() {
    let mut initialize = ScriptedResponse::response(
        "text/event-stream",
        "data: {\"id\":1,\"result\":{\"protocolVersion\":\"2025-03-26\"}}\n\n",
    );
    initialize
        .head
        .as_mut()
        .unwrap()
        .headers
        .push(("MCP-Session-ID".into(), "new-session".into()));
    let mut notification = ScriptedResponse::response("", "");
    notification.head.as_mut().unwrap().statusCode = 202;
    let tools = ScriptedResponse::response(
        "application/json",
        "{\"id\":2,\"result\":{\"tools\":[{\"name\":\"echo\"}]}}",
    );
    let mut no_events = ScriptedResponse::response("", "");
    no_events.head.as_mut().unwrap().statusCode = 405;
    let host = ResponseHost::with(vec![initialize, notification, no_events, tools]);
    let service = RegisteredService {
        name: "fixture".into(),
        serviceType: "remote".into(),
        command: String::new(),
        args: Vec::new(),
        cwd: None,
        endpoint: Some("https://test.invalid/mcp".into()),
        connectionType: Some("httpStream".into()),
        bearerToken: Some("fixture-token".into()),
        headers: BTreeMap::from([("X-Api-Key".into(), "fixture-key".into())]),
        description: String::new(),
        env: BTreeMap::new(),
        startupGate: Arc::new(tokio::sync::Mutex::new(())),
    };
    let active = startRemoteServiceSession(
        host.clone(),
        &service,
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    )
    .await
    .unwrap();
    assert!(active.ready);
    assert_eq!(active.tools[0]["name"], "echo");
    let requests = host.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    for (index, request) in requests.iter().enumerate() {
        assert!(request
            .headers
            .iter()
            .any(|(key, value)| key == "Authorization" && value == "Bearer fixture-token"));
        assert!(request
            .headers
            .iter()
            .any(|(key, value)| key == "X-Api-Key" && value == "fixture-key"));
        if index > 0 {
            assert!(request
                .headers
                .iter()
                .any(|(key, value)| key == "mcp-session-id" && value == "new-session"));
            assert!(request
                .headers
                .iter()
                .any(|(key, value)| key == "mcp-protocol-version" && value == "2025-03-26"));
        }
    }
    assert_eq!(host.cancelled.lock().unwrap().len(), 4);
}

/// Receives accepted requests on the already open GET channel without a second POST.
#[tokio::test]
async fn acceptedToolResultArrivesOnServerEvents() {
    let events = ScriptedResponse::response(
        "text/event-stream",
        concat!(
        "data: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\n",
        "data: {\"id\":99,\"result\":{}}\n\n",
        "data: {\"id\":7,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"异步结果\"}]}}\n\n"
    ),
    );
    let mut accepted = ScriptedResponse::response("", "");
    accepted.head.as_mut().unwrap().statusCode = 202;
    let host = ResponseHost::with(vec![events, accepted]);
    let mut session = session(host.clone());
    let deadline = StartupDeadline::new(testScheduler(), 1000).unwrap();
    connectServerEvents(&mut session, &deadline).await.unwrap();
    let result = sendJsonRpc(&mut session, request(7), Some(7), 1000)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["result"]["content"][0]["text"], "异步结果");
    assert_eq!(
        host.requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.method.as_str())
            .collect::<Vec<_>>(),
        ["GET", "POST"]
    );
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    drop(session);
    assert_eq!(host.cancelled.lock().unwrap().len(), 2);
}

/// Preserves the deadline and releases both streams when an accepted result never arrives.
#[tokio::test]
async fn acceptedResultTimeoutDoesNotReplayTheTool() {
    let events = ScriptedResponse::response("text/event-stream", ": heartbeat\n\n");
    let mut accepted = ScriptedResponse::response("", "");
    accepted.head.as_mut().unwrap().statusCode = 202;
    let host = ResponseHost::with(vec![events, accepted]);
    let mut session = session(host.clone());
    connectServerEvents(
        &mut session,
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    )
    .await
    .unwrap();
    let error = sendJsonRpc(&mut session, request(7), Some(7), 25)
        .await
        .unwrap_err();
    assert!(error.contains("timed out"), "{error}");
    assert_eq!(host.requests.lock().unwrap().len(), 2);
    drop(session);
    assert_eq!(host.cancelled.lock().unwrap().len(), 2);
}

/// Discovers all remote tools with cursor parameters and unique JSON-RPC request ids.
#[tokio::test]
async fn remoteToolsFollowEveryPage() {
    let init = ScriptedResponse::response(
        "application/json",
        "{\"id\":1,\"result\":{\"protocolVersion\":\"2025-03-26\"}}",
    );
    let mut notification = ScriptedResponse::response("", "");
    notification.head.as_mut().unwrap().statusCode = 202;
    let mut no_events = ScriptedResponse::response("", "");
    no_events.head.as_mut().unwrap().statusCode = 405;
    let first = ScriptedResponse::response(
        "application/json",
        "{\"id\":2,\"result\":{\"tools\":[{\"name\":\"first\"}],\"nextCursor\":\"page2\"}}",
    );
    let second = ScriptedResponse::response(
        "application/json",
        "{\"id\":3,\"result\":{\"tools\":[{\"name\":\"second\"}]}}",
    );
    let host = ResponseHost::with(vec![init, notification, no_events, first, second]);
    let mut active = ActiveService {
        scheduler: testScheduler(),
        process: None,
        remote: Some(session(host.clone())),
        requestId: 0,
        tools: Vec::new(),
        ready: false,
        logs: String::new(),
    };
    initializeRemoteService(
        &mut active,
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    )
    .await
    .unwrap();
    assert!(active.ready);
    assert_eq!(
        active
            .tools
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    let requests = host.requests.lock().unwrap();
    let last: Value = serde_json::from_slice(&requests.last().unwrap().body).unwrap();
    assert_eq!(last["params"]["cursor"], "page2");
    assert_eq!(last["id"], 3);
}

#[test]
fn decoderAcceptsLfCrLfAndCrAcrossChunkBoundaries() {
    for newline in ["\n", "\r\n", "\r"] {
        let wire = format!("data: {{\"id\":7,\"result\":{{\"text\":\"中文\"}}}}{newline}{newline}");
        let mut decoder = ResponseDecoder::new(true, 7);
        let mut decoded = None;
        for byte in wire.as_bytes() {
            if let Some(response) = decoder.push(&[*byte]).unwrap() {
                decoded = Some(response);
                break;
            }
        }
        assert_eq!(decoded.unwrap()["result"]["text"], "中文");
    }
}

#[test]
fn decoderRejectsOversizedOrMismatchedResponses() {
    let mut decoder = ResponseDecoder::new(false, 7);
    assert!(decoder
        .push(&vec![b' '; MAX_RESPONSE_BYTES + 1])
        .unwrap_err()
        .contains("16 MiB"));
    let mut decoder = ResponseDecoder::new(false, 7);
    assert!(decoder
        .push(b"{\"id\":8,\"result\":{}}")
        .unwrap_err()
        .contains("does not match"));
}

#[tokio::test]
async fn streamedNotificationsDoNotRequireBodyOrEof() {
    let host = ResponseHost::with(vec![ScriptedResponse::response("application/json", "")]);
    assert!(sendRemoteJsonRpc(
        &mut session(host.clone()),
        json!({"method":"notifications/initialized"}),
        None,
        1000
    )
    .await
    .unwrap()
    .is_none());
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn droppedPendingResponseCancelsItsHostStream() {
    let host = ResponseHost::with(vec![ScriptedResponse::response(
        "application/json",
        "{\"id\":7",
    )]);
    let mut session = session(host.clone());
    let mut future = Box::pin(sendJsonRpc(&mut session, request(7), Some(7), 1000));
    assert!(futures_util::poll!(&mut future).is_pending());
    drop(future);
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}
