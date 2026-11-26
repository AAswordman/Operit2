//! Exercises the real provider retry chain with a manually controlled Host deadline.

use std::sync::{Arc, Mutex};

use operit_providers::{chat, runtime_support};
#[path = "../../tests/ProviderMediaTestSupport.rs"]
mod runtime_fixture;

use operit_host_api::HostManager::{setDefaultHostRuntimeTaskSchedulerHost, setDefaultHttpHost};
use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpImageDelivery, HttpRequestData, HttpResponseData,
    HttpResponseHead, HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost,
    HttpStreamOpenedCallback, HttpStreamResponseCallback,
};
use operit_providers::chat::llmprovider::{
    AIService::{collect_response_text, AIService, SendMessageRequest},
    ClaudeProvider::ClaudeProvider,
    DeepseekProvider::DeepseekProvider,
    GeminiProvider::GeminiProvider,
    OpenAIProvider::OpenAIProvider,
    OpenAIResponsesProvider::OpenAIResponsesProvider,
};
use tokio::sync::{mpsc, oneshot};

struct Timer {
    delay_ms: u64,
    expired: oneshot::Sender<()>,
}

struct Scheduler(mpsc::UnboundedSender<Timer>);

impl HostRuntimeTaskSchedulerHost for Scheduler {
    /// Returns a fixed clock because this fixture explicitly advances Host delays.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(0)
    }

    /// Runs fixture tasks outside the test's coordination executor.
    fn scheduleHostRuntimeTask(&self, _: &str, task: HostRuntimeTask) -> HostResult<()> {
        std::thread::spawn(task);
        Ok(())
    }

    /// Creates each local provider future on the thread that owns its executor.
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(task());
        });
        Ok(())
    }

    /// Uses the same execution policy for cooperative fixture tasks.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, task)
    }

    /// Excludes synchronous delayed tasks from provider deadline and retry handling.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        panic!("provider must use asynchronous Host delays")
    }

    /// Exposes the exact production deadline and retry delays to deterministic test control.
    fn waitForHostRuntimeDelay(&self, delay_ms: u64) -> HostRuntimeTurnFuture {
        let (expired, elapsed) = oneshot::channel();
        self.0.send(Timer { delay_ms, expired }).unwrap();
        Box::pin(async move {
            elapsed
                .await
                .map_err(|_| HostError::new("fixture deadline controller dropped"))
        })
    }
}

struct PendingRequest {
    id: String,
    head: HttpStreamResponseCallback,
    chunk: HttpStreamChunkCallback,
    closed: HttpStreamClosedCallback,
}

impl PendingRequest {
    /// Delivers successful metadata without generating any model output.
    fn headers(&self, stream: bool) {
        (self.head)(HttpResponseHead {
            finalUrl: "http://127.0.0.1:8787/v1/chat/completions".into(),
            statusCode: 200,
            statusMessage: "OK".into(),
            headers: vec![(
                "Content-Type".into(),
                if stream {
                    "text/event-stream"
                } else {
                    "application/json"
                }
                .into(),
            )],
        });
    }

    /// Delivers bytes while keeping the socket open until the fixture explicitly closes it.
    fn bytes(&self, wire: &str) {
        (self.chunk)(wire.as_bytes().to_vec());
    }
}

struct HttpFixture {
    opened: mpsc::UnboundedSender<PendingRequest>,
    cancelled: Mutex<Vec<String>>,
}

impl HttpStreamHost for HttpFixture {
    /// Requires metadata streaming, matching the production provider transport.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("provider metadata transport required")
    }

    /// Accepts a request while deliberately withholding response headers and body bytes.
    fn openHttpResponseStream(
        &self,
        id: String,
        _: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.opened
            .send(PendingRequest {
                id,
                head,
                chunk,
                closed,
            })
            .unwrap();
        Ok(())
    }

    /// Records cancellation before the next request is allowed to start.
    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        self.cancelled.lock().unwrap().push(id.into());
        Ok(())
    }
}

impl HttpHost for HttpFixture {
    /// Declares in-memory image delivery for this network-only fixture.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }

    /// Rejects blocking transport on the asynchronous provider path.
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        panic!("async transport required")
    }

    /// Excludes unrelated downloads from the fixture.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("unused"))
    }
}

#[derive(Clone, Copy, Debug)]
enum Protocol {
    OpenAi,
    Claude,
    Gemini,
    Deepseek,
    Responses,
}

impl Protocol {
    /// Constructs a real provider using its normal cancellation, parsing and retry implementation.
    fn provider(self) -> Box<dyn AIService> {
        let endpoint = "http://127.0.0.1:8787/v1/chat/completions".to_string();
        match self {
            Self::OpenAi => Box::new(OpenAIProvider::new(
                endpoint,
                "".into(),
                "test".into(),
                "OPENAI_GENERIC".into(),
                vec![],
                true,
            )),
            Self::Claude => Box::new(ClaudeProvider::new(
                endpoint,
                "".into(),
                "test".into(),
                "CLAUDE".into(),
                vec![],
                true,
            )),
            Self::Gemini => Box::new(GeminiProvider::new(
                endpoint,
                "".into(),
                "test".into(),
                "GEMINI".into(),
                vec![],
                vec![],
                true,
            )),
            Self::Deepseek => Box::new(DeepseekProvider::new(
                endpoint, "".into(), "test".into(), "DEEPSEEK".into(), vec![],
                false, false, false, vec![], true, runtime_fixture::test_runtime_context(),
            )),
            Self::Responses => Box::new(OpenAIResponsesProvider::new(
                endpoint, "".into(), "test".into(), "OPENAI_RESPONSES".into(), vec![],
                false, false, false, vec![], true, runtime_fixture::test_runtime_context(),
            )),
        }
    }

    /// Returns real text, reasoning or native tool deltas in each provider's wire format.
    fn output(self, kind: usize) -> &'static str {
        match (self, kind) {
            (Self::Deepseek, 3) => "data: {\"choices\":[{\"delta\":{\"content\":\"<｜｜DSML｜｜calls><｜｜DSML｜｜invoke name=\\\"echo\\\"><｜｜DSML｜｜parameter name=\\\"text\\\" string=\\\"true\\\">partial\"}}]}\n\n",
            (Self::OpenAi | Self::Deepseek, 0) => "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
            (Self::OpenAi | Self::Deepseek, 1) => "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}\n\n",
            (Self::OpenAi | Self::Deepseek, _) => "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"echo\",\"arguments\":\"{}\"}}]}}]}\n\n",
            (Self::Responses, 0) => "data: {\"type\":\"response.output_text.delta\",\"delta\":\"ok\"}\n\n",
            (Self::Responses, 1) => "data: {\"type\":\"response.reasoning_text.delta\",\"delta\":\"thinking\"}\n\n",
            (Self::Responses, _) => "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"call_id\":\"call_fixture\",\"name\":\"echo\",\"arguments\":\"{}\"}}\n\n",
            (Self::Claude, 0) => "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n",
            (Self::Claude, 1) => concat!("data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n", "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"thinking\"}}\n\n"),
            (Self::Claude, _) => "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"tool_use\",\"name\":\"echo\",\"input\":{}}}\n\n",
            (Self::Gemini, 0) => "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}\n\n",
            (Self::Gemini, 1) => "data: {\"candidates\":[{\"content\":{\"parts\":[{\"thought\":true,\"text\":\"thinking\"}]}}]}\n\n",
            (Self::Gemini, _) => "data: {\"candidates\":[{\"content\":{\"parts\":[{\"functionCall\":{\"name\":\"echo\",\"args\":{}}}]}}]}\n\n",
        }
    }

    /// Supplies valid protocol metadata that must not release the first-output deadline.
    fn metadata(self) -> &'static str {
        match self {
            Self::OpenAi | Self::Deepseek => concat!(
                "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
                "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":70000,\"completion_tokens\":0}}\n\n"
            ),
            Self::Claude => concat!(
                "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":70000}}}\n\n",
                "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n"
            ),
            Self::Gemini => "data: {\"candidates\":[{\"content\":{\"parts\":[{\"thought\":true,\"text\":\"\",\"thoughtSignature\":\"signature\"}]}}],\"usageMetadata\":{\"promptTokenCount\":70000}}\n\n",
            Self::Responses => "data: {\"type\":\"response.created\"}\n\n",
        }
    }

    /// Returns a complete buffered response when streaming is disabled.
    fn json(self) -> &'static str {
        match self {
            Self::OpenAi | Self::Deepseek => "{\"choices\":[{\"message\":{\"content\":\"ok\"}}]}",
            Self::Claude => "{\"content\":[{\"type\":\"text\",\"text\":\"ok\"}]}",
            Self::Gemini => "{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]}}]}",
            Self::Responses => "{\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"ok\"}]}]}",
        }
    }

    /// Completes the model protocol without depending on a remote EOF.
    fn completion(self) -> &'static str {
        match self {
            Self::OpenAi | Self::Deepseek => "data: [DONE]\n\n",
            Self::Claude => "data: {\"type\":\"message_stop\"}\n\n",
            Self::Gemini => "data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n\n",
            Self::Responses => "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[]}}\n\n",
        }
    }
}

struct Harness {
    http: Arc<HttpFixture>,
    requests: mpsc::UnboundedReceiver<PendingRequest>,
    timers: mpsc::UnboundedReceiver<Timer>,
}

/// Registers the process-wide Host contracts once for all sequential cases.
fn fixture() -> Harness {
    let (opened, requests) = mpsc::unbounded_channel();
    let (scheduled, timers) = mpsc::unbounded_channel();
    let http = Arc::new(HttpFixture {
        opened,
        cancelled: Mutex::new(Vec::new()),
    });
    setDefaultHttpHost(http.clone());
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(Scheduler(scheduled)));
    Harness {
        http,
        requests,
        timers,
    }
}

/// Builds the normal request contract with explicit retry and streaming preferences.
fn request(stream: bool, retry: bool, errors: Arc<Mutex<Vec<String>>>) -> SendMessageRequest {
    SendMessageRequest {
        chat_history: vec![],
        model_parameters: vec![],
        thinking: Default::default(),
        stream,
        available_tools: vec![],
        preserve_think_in_history: false,
        enable_retry: retry,
        on_non_fatal_error: Some(Arc::new(move |error| errors.lock().unwrap().push(error))),
        on_tool_invocation: None,
    }
}

/// Reproduces the user's missing-header wait, body stalls and heartbeat-only streams through retry.
async fn retries_after_first_output_timeout(
    harness: &mut Harness,
    protocol: Protocol,
    stream: bool,
    with_headers: bool,
) {
    let Harness {
        http,
        requests,
        timers,
    } = harness;
    http.cancelled.lock().unwrap().clear();
    let errors = Arc::new(Mutex::new(Vec::new()));
    let mut provider = protocol.provider();
    let mut output = provider
        .send_message(request(stream, true, errors.clone()))
        .await
        .unwrap();
    let collect = async {
        let mut text = String::new();
        output.collect(&mut |chunk| text.push_str(&chunk)).await;
        (text, output.terminal_failure())
    };
    let drive = async {
        let deadline = timers.recv().await.unwrap();
        assert_eq!(deadline.delay_ms, 120_000);
        let first = requests.recv().await.unwrap();
        if with_headers {
            first.headers(stream);
            if stream {
                first.bytes(": heartbeat\n\ndata: {}\n\n");
                first.bytes(protocol.metadata());
            }
        }
        deadline.expired.send(()).unwrap();
        let retry = timers.recv().await.unwrap();
        assert_eq!(retry.delay_ms, 1000);
        assert_eq!(*http.cancelled.lock().unwrap(), [first.id]);
        assert_eq!(
            errors.lock().unwrap().as_slice(),
            ["AI 首字等待超过 120 秒，正在进行第 1 次重试..."]
        );
        retry.expired.send(()).unwrap();
        let deadline = timers.recv().await.unwrap();
        assert_eq!(deadline.delay_ms, 120_000);
        let second = requests.recv().await.unwrap();
        second.headers(stream);
        second.bytes(if stream {
            protocol.output(0)
        } else {
            protocol.json()
        });
        if stream {
            second.bytes(protocol.completion());
        }
        (second.closed)(Ok(()));
    };
    let ((text, failure), ()) = futures_util::future::join(collect, drive).await;
    assert_eq!(failure, None, "{protocol:?} stream={stream}");
    assert!(text.contains("ok"), "{protocol:?}: {text}");
    assert_eq!(http.cancelled.lock().unwrap().len(), 2);
}

/// Confirms real output cancels only the first-output timer while cancellation remains responsive.
async fn output_disarms_deadline(harness: &mut Harness, protocol: Protocol, kind: usize) {
    let Harness {
        http,
        requests,
        timers,
    } = harness;
    http.cancelled.lock().unwrap().clear();
    let mut provider = protocol.provider();
    let mut output = provider
        .send_message(request(true, true, Arc::default()))
        .await
        .unwrap();
    let collect = async {
        output.collect(&mut |_| {}).await;
        output.terminal_failure()
    };
    let drive = async {
        let mut deadline = timers.recv().await.unwrap();
        let first = requests.recv().await.unwrap();
        first.headers(true);
        first.bytes(protocol.output(kind));
        deadline.expired.closed().await;
        provider.cancel_streaming();
    };
    let (failure, ()) = futures_util::future::join(collect, drive).await;
    assert_eq!(failure, None);
    assert_eq!(http.cancelled.lock().unwrap().len(), 1);
    assert!(timers.try_recv().is_err());
}

/// Preserves disabled retries, retry exhaustion and cancellation before the first response header.
async fn terminal_cases(harness: &mut Harness, protocol: Protocol, retries: bool, cancel: bool) {
    let Harness {
        http,
        requests,
        timers,
    } = harness;
    http.cancelled.lock().unwrap().clear();
    let errors = Arc::new(Mutex::new(Vec::new()));
    let mut provider = protocol.provider();
    let mut output = provider
        .send_message(request(true, retries, errors.clone()))
        .await
        .unwrap();
    let collect = async {
        output.collect(&mut |_| {}).await;
        output.terminal_failure()
    };
    let drive = async {
        let attempts = if retries && !cancel { 6 } else { 1 };
        let mut pending = Vec::new();
        for attempt in 0..attempts {
            let mut deadline = timers.recv().await.unwrap();
            assert_eq!(deadline.delay_ms, 120_000);
            let opened = requests.recv().await.unwrap();
            if cancel {
                provider.cancel_streaming();
                deadline.expired.closed().await;
            } else {
                deadline.expired.send(()).unwrap();
            }
            if attempt + 1 < attempts {
                let retry = timers.recv().await.unwrap();
                assert_eq!(retry.delay_ms, 1000 << attempt);
                assert!(http.cancelled.lock().unwrap().contains(&opened.id));
                retry.expired.send(()).unwrap();
            }
            pending.push(opened);
        }
        (attempts, pending)
    };
    let (failure, (attempts, _pending)) = futures_util::future::join(collect, drive).await;
    if cancel {
        assert_eq!(failure, None);
    } else {
        assert_eq!(
            failure.as_deref(),
            Some("AI first output timed out after 120 seconds")
        );
    }
    assert_eq!(http.cancelled.lock().unwrap().len(), attempts);
    assert_eq!(errors.lock().unwrap().len(), attempts - 1);
    assert!(timers.try_recv().is_err());
}

/// Replays real output followed by a disconnect through the collector used by summaries.
async fn retries_discard_partial_output(harness: &mut Harness, protocol: Protocol, kind: usize) {
    harness.http.cancelled.lock().unwrap().clear();
    let mut provider = protocol.provider();
    let output = provider.send_message(request(true, true, Arc::default())).await.unwrap();
    let collect = collect_response_text(output);
    let drive = async {
        let mut deadline = harness.timers.recv().await.unwrap();
        let first = harness.requests.recv().await.unwrap();
        first.headers(true);
        first.bytes(protocol.output(kind));
        deadline.expired.closed().await;
        (first.closed)(Err("fixture connection interrupted".into()));
        let retry = harness.timers.recv().await.unwrap();
        assert_eq!(retry.delay_ms, 1000);
        retry.expired.send(()).unwrap();
        let _deadline = harness.timers.recv().await.unwrap();
        let second = harness.requests.recv().await.unwrap();
        second.headers(true);
        second.bytes(protocol.output(0));
        second.bytes(protocol.completion());
        (second.closed)(Ok(()));
    };
    let (text, ()) = futures_util::future::join(collect, drive).await;
    assert_eq!(text.unwrap().trim(), "ok", "{protocol:?} kind={kind}");
}

/// Runs serially because each case intentionally owns the process-wide Host contracts.
#[tokio::test]
async fn model_first_output_deadline_covers_missing_headers_and_retries() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut harness = fixture();
        for protocol in [Protocol::OpenAi, Protocol::Claude, Protocol::Gemini] {
            for stream in [false, true] {
                for with_headers in [false, true] {
                    retries_after_first_output_timeout(
                        &mut harness,
                        protocol,
                        stream,
                        with_headers,
                    )
                    .await;
                }
            }
            for kind in 0..3 {
                output_disarms_deadline(&mut harness, protocol, kind).await;
                retries_discard_partial_output(&mut harness, protocol, kind).await;
            }
            terminal_cases(&mut harness, protocol, false, false).await;
            terminal_cases(&mut harness, protocol, true, false).await;
            terminal_cases(&mut harness, protocol, true, true).await;
        }
        for protocol in [Protocol::Deepseek, Protocol::Responses] {
            for kind in 0..3 {
                retries_discard_partial_output(&mut harness, protocol, kind).await;
            }
        }
        retries_discard_partial_output(&mut harness, Protocol::Deepseek, 3).await;
    })
    .await
    .expect("Host-controlled deadlines and retries must settle without real-time waits");
}
