#![cfg(feature = "http-agent")]
use operit_edge_sdk::{agent::*, http_agent::*};
use operit_host_api::*;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
struct Http {
    body: Vec<u8>,
    requests: Mutex<Vec<Value>>,
    closed: AtomicUsize,
}
impl HttpStreamHost for Http {
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        unreachable!()
    }
    fn openHttpResponseStream(
        &self,
        _: String,
        request: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        close: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        assert!(request
            .headers
            .contains(&("Authorization".into(), "Bearer key".into())));
        assert!(!request.followRedirects);
        self.requests
            .lock()
            .unwrap()
            .push(serde_json::from_slice(&request.body).unwrap());
        head(HttpResponseHead {
            finalUrl: request.url,
            statusCode: 200,
            statusMessage: "OK".into(),
            headers: vec![],
        });
        for bytes in self.body.chunks(7) {
            chunk(bytes.to_vec());
        }
        close(Ok(()));
        Ok(())
    }
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        self.closed.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
impl HttpHost for Http {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        unreachable!()
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
fn backend(body: Value, response_limit: usize) -> (HttpBackend, Arc<Http>) {
    let http = Arc::new(Http {
        body: serde_json::to_vec(&body).unwrap(),
        requests: Mutex::new(vec![]),
        closed: AtomicUsize::new(0),
    });
    let backend = HttpBackend::new(
        http.clone(),
        HttpAgentConfig {
            endpoint: "http://127.0.0.1/model".into(),
            model: "model".into(),
            api_key: "key".into(),
            max_response_bytes: response_limit,
            max_request_bytes: 4096,
            timeout_seconds: 5,
        },
    )
    .unwrap();
    (backend, http)
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}
#[test]
fn streamed_provider_reply_and_tool_observations_preserve_the_protocol() {
    let (backend, http) = backend(
        json!({"choices":[{"message":{"content":"温度23.5🙂"}}]}),
        1024,
    );
    let observations = [Observation {
        tool: "sensor".into(),
        arguments: "{}".into(),
        result: "23.5".into(),
    }];
    let result = runtime()
        .block_on(backend.next(Request {
            instruction: "sensor",
            prompt: "read",
            tools: &[],
            observations: &observations,
        }))
        .unwrap();
    assert!(matches!(result, Step::Reply(text) if text == "温度23.5🙂"));
    let request = &http.requests.lock().unwrap()[0];
    assert_eq!(
        request["messages"][2]["tool_calls"][0]["function"]["name"],
        "sensor"
    );
    assert_eq!(request["messages"][3]["content"], "23.5");
    assert_eq!(http.closed.load(Ordering::Relaxed), 1);
}
#[test]
fn response_overflow_closes_the_stream_without_returning_partial_text() {
    let (backend, http) = backend(
        json!({"choices":[{"message":{"content":"x".repeat(300)}}]}),
        128,
    );
    let result = runtime().block_on(backend.next(Request {
        instruction: "",
        prompt: "",
        tools: &[],
        observations: &[],
    }));
    assert!(matches!(result, Err(AgentError::OutputTooLarge)));
    assert_eq!(http.closed.load(Ordering::Relaxed), 1);
}
#[test]
fn structured_tool_calls_reach_the_device_loop() {
    let (backend, _) = backend(
        json!({"choices":[{"message":{"tool_calls":[{"function":{"name":"sensor","arguments":"{}"}}]}}]}),
        1024,
    );
    let result = runtime()
        .block_on(backend.next(Request {
            instruction: "",
            prompt: "",
            tools: &[],
            observations: &[],
        }))
        .unwrap();
    assert!(
        matches!(result, Step::Call { tool, arguments } if tool == "sensor" && arguments == "{}")
    );
}
