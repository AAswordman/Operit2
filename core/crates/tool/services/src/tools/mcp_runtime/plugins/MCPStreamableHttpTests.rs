use super::*;
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpImageDelivery, HttpStreamChunkCallback, HttpStreamClosedCallback,
    HttpStreamHost, HttpStreamOpenedCallback, HttpStreamResponseCallback,
};
use std::collections::VecDeque;
use std::time::Instant;

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
        assert_eq!(request.method, "POST");
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
        sseStreamId: None,
    }
}

fn request(id: u64) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/list"})
}

#[test]
fn heldOpenSseReturnsMatchingResultWithoutEofAndCancels() {
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
        .unwrap()
        .unwrap();
    assert_eq!(response["result"]["text"], "实时读取");
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    assert!(host.streams.lock().unwrap().is_empty());
}

#[test]
fn heldOpenJsonReturnsAsSoonAsComplete() {
    let host = ResponseHost::with(vec![ScriptedResponse::response(
        "application/json",
        "{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}",
    )]);
    assert!(
        sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 1000)
            .unwrap()
            .is_some()
    );
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[test]
fn deadlinesCoverHeadersAndIncompleteBodies() {
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
        let error = sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 25).unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    }
}

#[test]
fn notificationNeedsOnlyAcceptedStatusAndStillCancelsBody() {
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
        .unwrap(),
        None
    );
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[test]
fn failuresNeverReplayAndAlwaysCancel() {
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
        let error = sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 1000).unwrap_err();
        assert!(error.contains(needle), "{error}");
        assert_eq!(host.requests.lock().unwrap().len(), 1);
        assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    }
}

#[test]
fn prematureEofAndConnectionErrorsFailImmediately() {
    for (closed, needle) in [
        (Ok(()), "ended before response"),
        (Err("connection lost".into()), "connection lost"),
    ] {
        let mut response = ScriptedResponse::response("text/event-stream", ": heartbeat\n\n");
        response.closed = Some(closed);
        let host = ResponseHost::with(vec![response]);
        let started = Instant::now();
        let error = sendJsonRpc(&mut session(host.clone()), request(7), Some(7), 1000).unwrap_err();
        assert!(error.contains(needle), "{error}");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(host.cancelled.lock().unwrap().len(), 1);
    }
}

#[test]
fn httpErrorKeepsStatusAndBoundedDiagnosticsWithoutPoisoningSession() {
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
    let error = sendJsonRpc(&mut session, request(7), Some(7), 1000).unwrap_err();
    assert!(error.contains("status 401"), "{error}");
    assert!(error.contains("authentication"), "{error}");
    assert_eq!(session.sessionId.as_deref(), Some("valid-session"));
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

#[test]
fn sessionAndNegotiatedVersionSurviveIncrementalHandshake() {
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
    let host = ResponseHost::with(vec![initialize, notification, tools]);
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
    };
    let active = startRemoteServiceSession(
        host.clone(),
        &service,
        &StartupDeadline::new(testScheduler(), 1000).unwrap(),
    )
    .unwrap();
    assert!(active.ready);
    assert_eq!(active.tools[0]["name"], "echo");
    let requests = host.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
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
    assert_eq!(host.cancelled.lock().unwrap().len(), 3);
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

struct BufferedHost {
    body: String,
}
impl HttpStreamHost for BufferedHost {
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("buffered Hosts must not be forced onto callback channels");
    }
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        panic!("no stream was opened");
    }
}
impl HttpHost for BufferedHost {
    fn responseDelivery(&self) -> operit_host_api::HttpResponseDelivery {
        operit_host_api::HttpResponseDelivery::Buffered
    }
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        Ok(HttpResponseData {
            finalUrl: request.url,
            statusCode: 200,
            statusMessage: "OK".into(),
            headers: Vec::new(),
            body: self.body.as_bytes().to_vec(),
        })
    }
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        unreachable!()
    }
}

#[test]
fn transportSelectionUsesHostCapabilitiesNotCompilationTarget() {
    let host = Arc::new(BufferedHost {
        body: json!({"jsonrpc":"2.0", "id":7, "result":{"text":"buffered"}}).to_string(),
    });
    let response = sendRemoteJsonRpc(&mut session(host), request(7), Some(7), 1000)
        .unwrap()
        .unwrap();
    assert_eq!(response["id"], 7);
    assert_eq!(response["result"]["text"], "buffered");
    let streamed = ResponseHost::with(vec![ScriptedResponse::response(
        "application/json",
        "{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}",
    )]);
    assert!(
        sendRemoteJsonRpc(&mut session(streamed), request(7), Some(7), 1000)
            .unwrap()
            .is_some()
    );
}

#[test]
fn bufferedHostDoesNotAcknowledgeWrongOrMissingResponses() {
    for body in ["", "{\"jsonrpc\":\"2.0\",\"id\":8,\"result\":{}}"] {
        let host = Arc::new(BufferedHost { body: body.into() });
        assert!(
            sendRemoteJsonRpc(&mut session(host), request(7), Some(7), 1000)
                .unwrap_err()
                .contains("response 7")
        );
    }
}

#[test]
fn bufferedNotificationsOnlyRequireSuccessfulHttpStatus() {
    let host = Arc::new(BufferedHost { body: "".into() });
    assert!(sendRemoteJsonRpc(
        &mut session(host),
        json!({"method":"notifications/initialized"}),
        None,
        1000
    )
    .unwrap()
    .is_none());
}

#[test]
fn finiteResponseHostRejectsLiveSseBeforeOpeningAStream() {
    let host = Arc::new(BufferedHost { body: "".into() });
    assert!(connectRemoteSse(&mut session(host), 1000)
        .unwrap_err()
        .contains("not live MCP SSE"));
}
