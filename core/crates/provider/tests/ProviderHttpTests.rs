use super::*;
use crate::chat::llmprovider::AIService::{AIService, SharedAiResponseStream};
use crate::chat::llmprovider::ClaudeProvider::ClaudeProvider;
use crate::chat::llmprovider::GeminiProvider::GeminiProvider;
use crate::chat::llmprovider::OpenAIProvider::{OpenAIProvider, StreamEmitter};
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpImageDelivery, HttpResponseData, HttpStreamChunkCallback,
    HttpStreamClosedCallback, HttpStreamHost, HttpStreamOpenedCallback, HttpStreamResponseCallback,
};
use std::sync::Mutex;

#[derive(Default)]
struct ControlledHost {
    initial: Vec<u8>,
    callbacks: Mutex<Option<(HttpStreamChunkCallback, HttpStreamClosedCallback)>>,
    requests: Mutex<Vec<HttpRequestData>>,
    cancelled: Mutex<Vec<String>>,
}

impl ControlledHost {
    /// Queues initial body bytes while leaving the response open under test control.
    fn new(initial: &str) -> Arc<Self> {
        Arc::new(Self {
            initial: initial.as_bytes().to_vec(),
            ..Self::default()
        })
    }

    /// Delivers bytes after a provider is already waiting for its next event.
    fn emit(&self, wire: &str) {
        let callback = self.callbacks.lock().unwrap().as_ref().unwrap().0.clone();
        callback(wire.as_bytes().to_vec());
    }

    /// Closes the transport without asserting that a model completed normally.
    fn finish(&self) {
        let callback = self.callbacks.lock().unwrap().as_ref().unwrap().1.clone();
        callback(Ok(()));
    }
}

impl HttpStreamHost for ControlledHost {
    /// Requires metadata streaming so HTTP errors remain visible to providers.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("metadata required")
    }

    /// Emits headers and bytewise UTF-8 data through the production Host callback contract.
    fn openHttpResponseStream(
        &self,
        _: String,
        request: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.requests.lock().unwrap().push(request);
        *self.callbacks.lock().unwrap() = Some((chunk.clone(), closed));
        head(HttpResponseHead {
            finalUrl: "https://fixture.invalid/model".to_string(),
            statusCode: 200,
            statusMessage: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/event-stream".to_string())],
        });
        for byte in &self.initial {
            chunk(vec![*byte]);
        }
        Ok(())
    }

    /// Records cancellation independently of whether the remote peer sent EOF.
    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        self.cancelled.lock().unwrap().push(id.to_string());
        Ok(())
    }
}

impl HttpHost for ControlledHost {
    /// Declares byte-delivered image support for the fixture Host.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    /// Rejects synchronous networking from an asynchronous model request.
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        panic!("stream required")
    }
    /// Excludes file downloading from the provider transport fixture.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("not used"))
    }
}

/// Creates an ordered output whose live content can be inspected before completion.
fn output() -> SharedAiResponseStream {
    SharedAiResponseStream::new_ordered(
        operit_util::stream::HotStream::mutable_shared_stream(usize::MAX),
        operit_util::stream::HotStream::mutable_shared_stream(usize::MAX),
    )
}

/// Creates a Claude provider without registering a global network or timer Host.
fn claude() -> ClaudeProvider {
    ClaudeProvider::new(
        "https://fixture.invalid".to_string(),
        String::new(),
        "test".to_string(),
        "CLAUDE".to_string(),
        Vec::new(),
        true,
    )
}

/// Creates a Gemini provider without registering a global network or timer Host.
fn gemini() -> GeminiProvider {
    GeminiProvider::new(
        "https://fixture.invalid".to_string(),
        String::new(),
        "test".to_string(),
        "GEMINI".to_string(),
        Vec::new(),
        Vec::new(),
        true,
    )
}

/// Proves that Claude text reaches observers before the upstream sends its final event or EOF.
#[tokio::test]
async fn claude_emits_before_completion_and_stops_without_eof() {
    let host = ControlledHost::new("data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"你好😀\"}}\n\n");
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    let mut provider = claude();
    let output = output();
    let mut read = Box::pin(provider.process_streaming_response(response, &output));
    assert!(futures_util::poll!(&mut read).is_pending());
    assert_eq!(output.current_render_content(), "你好😀");
    host.emit("data: {\"type\":\"message_stop\"}\n\n");
    read.await.unwrap();
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

/// Preserves escaped emoji across Claude tool-input events before message completion.
#[tokio::test]
async fn claude_streams_tool_arguments_with_split_surrogate_pairs() {
    let host = ControlledHost::new("");
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    host.emit(
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_test\",\"name\":\"echo\",\"input\":{}}}\n\n",
    );
    let mut provider = claude();
    let output = output();
    let mut read = Box::pin(provider.process_streaming_response(response, &output));
    assert!(futures_util::poll!(&mut read).is_pending());
    for fragment in [r#"{"text":"hello\uD8"#, r#"3D\uDE"#, r#"00"}"#] {
        let event = serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "input_json_delta", "partial_json": fragment}
        });
        host.emit(&format!("data: {event}\n\n"));
        assert!(futures_util::poll!(&mut read).is_pending());
    }
    let rendered = output.current_render_content();
    assert!(
        rendered.contains("<param name=\"text\">hello😀</param>"),
        "unexpected tool output: {rendered}"
    );
    host.emit(concat!(
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    ));
    read.await.unwrap();
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

/// Rejects a clean socket EOF that lacks Claude's authoritative completion marker.
#[tokio::test]
async fn claude_rejects_truncation_and_keeps_partial_output() {
    let host = ControlledHost::new(
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"partial\"}}\n\n",
    );
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    host.finish();
    let output = output();
    let error = claude()
        .process_streaming_response(response, &output)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("before completion"));
    assert_eq!(output.current_render_content(), "partial");
}

/// Reports Anthropic overload errors rather than marking a partial answer successful.
#[tokio::test]
async fn claude_propagates_upstream_error_events() {
    let host = ControlledHost::new("data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"busy\"}}\n\n");
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    let error = claude()
        .process_streaming_response(response, &output())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("overloaded_error"));
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

/// Proves Gemini emits live text and ends on finishReason even when HTTP stays open.
#[tokio::test]
async fn gemini_emits_before_completion_and_stops_without_eof() {
    let host = ControlledHost::new(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"live\"}]}}]}\n\n",
    );
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    let mut provider = gemini();
    let output = output();
    let mut read = Box::pin(provider.process_streaming_response(response, &output));
    assert!(futures_util::poll!(&mut read).is_pending());
    assert_eq!(output.current_render_content(), "live");
    host.emit("data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n\n");
    read.await.unwrap();
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

/// Treats Gemini EOF without finishReason as interruption, preserving emitted text.
#[tokio::test]
async fn gemini_rejects_truncation_and_keeps_partial_output() {
    let host = ControlledHost::new(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"partial\"}]}}]}\n\n",
    );
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    host.finish();
    let output = output();
    let error = gemini()
        .process_streaming_response(response, &output)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("before completion"));
    assert_eq!(output.current_render_content(), "partial");
}

/// Cancels a provider while its Host response is idle, without waiting for another byte.
#[tokio::test]
async fn cancelling_idle_claude_releases_the_host_stream() {
    let host = ControlledHost::new("");
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    let mut provider = claude();
    let mut control = provider.clone();
    let output = output();
    let mut read = Box::pin(provider.process_streaming_response(response, &output));
    assert!(futures_util::poll!(&mut read).is_pending());
    control.cancel_streaming();
    assert_eq!(read.await.unwrap_err(), AiServiceError::RequestCancelled);
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

/// Checks model POSTs and OAuth forms share the exact same Host-owned network policy.
#[tokio::test]
async fn model_and_oauth_requests_use_the_host_policy() {
    let host = ControlledHost::new("");
    drop(
        post("https://fixture.invalid")
            .json(&serde_json::json!({"stream":true}))
            .send_with_host(host.clone())
            .await
            .unwrap(),
    );
    drop(
        post("https://fixture.invalid/token")
            .form(&[("code", "a +&中")])
            .send_with_host(host.clone())
            .await
            .unwrap(),
    );
    let requests = host.requests.lock().unwrap();
    assert_eq!(
        requests[0].connectTimeoutSeconds,
        requests[1].connectTimeoutSeconds
    );
    assert_eq!(requests[0].readTimeoutSeconds, 1000);
    assert_eq!(requests[0].ignoreSsl, requests[1].ignoreSsl);
    assert_eq!(
        String::from_utf8(requests[1].body.clone()).unwrap(),
        "code=a+%2B%26%E4%B8%AD"
    );
    assert_eq!(host.cancelled.lock().unwrap().len(), 2);
}

/// Frames multiline JSON, ignores empty events, and continues after an invalid JSON event.
#[tokio::test]
async fn openai_accepts_multiline_events_and_logs_invalid_json() {
    let host = ControlledHost::new(concat!(
        ": heartbeat\r\n\r\ndata:\r\n\r\n",
        "data: not-json\r\n\r\n",
        "data: {\"choices\":[{\"delta\":\r\n",
        "data: {\"content\":\"完整😀\"}}]}\r\n\r\n",
        "data: [DONE]\r\n\r\n"
    ));
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    let provider = OpenAIProvider::new(
        "https://fixture.invalid".to_string(),
        String::new(),
        "test".to_string(),
        "OPENAI_GENERIC".to_string(),
        Vec::new(),
        true,
    );
    let output = output();
    let mut emitter = StreamEmitter::new(output.clone());
    provider
        .read_streaming_response(response, None, &output, &mut emitter)
        .await
        .unwrap();
    assert_eq!(output.current_render_content(), "完整😀");
    assert_eq!(host.cancelled.lock().unwrap().len(), 1);
}

/// Invalid events cannot turn an unfinished OpenAI response into a successful completion.
#[tokio::test]
async fn openai_invalid_event_still_requires_completion() {
    let host = ControlledHost::new("data: not-json\n\n");
    let response = post("https://fixture.invalid")
        .send_with_host(host.clone())
        .await
        .unwrap();
    host.finish();
    let provider = OpenAIProvider::new(
        "https://fixture.invalid".to_string(),
        String::new(),
        "test".to_string(),
        "OPENAI_GENERIC".to_string(),
        Vec::new(),
        true,
    );
    let output = output();
    let error = provider
        .read_streaming_response(
            response,
            None,
            &output,
            &mut StreamEmitter::new(output.clone()),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("before completion"));
}
