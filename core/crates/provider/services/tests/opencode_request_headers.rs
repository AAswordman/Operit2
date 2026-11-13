//! Verifies OpenCode identity headers at the actual provider HTTP Host boundary.

#![allow(non_snake_case, dead_code)]

use operit_model::ModelConfigData::ApiProviderType;
use operit_providers::chat::llmprovider::AIService::AIService;
use operit_providers::chat::llmprovider::OpenCodeProvider::OpenCodeProvider;
use operit_providers::{chat, runtime_support};

#[path = "../../tests/ProviderMediaTestSupport.rs"]
mod fixtures;
use crate::chat::llmprovider::AIServiceFactory::{
    AIServiceFactory, ProviderCreateParams, ProviderCreateRequest,
};
use fixtures::{send_request, test_runtime_context};
use operit_host_api::HostManager::{setDefaultHostRuntimeTaskSchedulerHost, setDefaultHttpHost};
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpImageDelivery, HttpRequestData, HttpResponseData,
    HttpResponseHead, HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost,
    HttpStreamOpenedCallback, HttpStreamResponseCallback,
};
use operit_model::ModelConfigData::ResolvedModelConfig;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct CapturingHost(Mutex<Vec<HttpRequestData>>);

struct TestScheduler(std::time::Instant);

impl operit_host_api::HostRuntimeTaskSchedulerHost for TestScheduler {
    /// Reads elapsed fixture time for provider diagnostics.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(self.0.elapsed().as_millis() as u64)
    }

    /// Executes synchronous fixture work on a separate thread.
    fn scheduleHostRuntimeTask(
        &self,
        _: &str,
        task: operit_host_api::HostRuntimeTask,
    ) -> HostResult<()> {
        std::thread::spawn(task);
        Ok(())
    }

    /// Creates each local provider future on the executor that owns it.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        _: &str,
        task: operit_host_api::HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(task());
        });
        Ok(())
    }

    /// Dispatches cooperative fixture work on the same test execution policy.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: operit_host_api::HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, task)
    }

    /// Excludes retry scheduling from the header regression fixture.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: operit_host_api::HostRuntimeTask,
    ) -> HostResult<()> {
        panic!("header fixture must not retry")
    }

    /// Provides a completed turn boundary for this deterministic fixture.
    fn waitForHostRuntimeTaskTurn(&self) -> operit_host_api::HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }

    /// Excludes retry delays from the header regression fixture.
    fn waitForHostRuntimeDelay(&self, _: u64) -> operit_host_api::HostRuntimeTurnFuture {
        panic!("header fixture must not retry")
    }
}

impl HttpStreamHost for CapturingHost {
    /// Requires the production response metadata contract.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("OpenCode requests must use metadata streaming")
    }

    /// Captures the fully encoded request and rejects it before response parsing.
    fn openHttpResponseStream(
        &self,
        _: String,
        request: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        let final_url = request.url.clone();
        self.0.lock().unwrap().push(request);
        head(HttpResponseHead {
            finalUrl: final_url,
            statusCode: 400,
            statusMessage: "Bad Request".into(),
            headers: Vec::new(),
        });
        chunk(b"header fixture".to_vec());
        closed(Ok(()));
        Ok(())
    }

    /// Completes the provider's response cleanup without opening a socket.
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        Ok(())
    }
}

impl HttpHost for CapturingHost {
    /// Declares the fixture's image transport capability.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }

    /// Rejects synchronous transport for model requests.
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        panic!("model requests must use response streaming")
    }

    /// Excludes unrelated downloads from this fixture.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("unused"))
    }
}

/// Creates a resolved OpenCode configuration with conflicting client headers.
fn config(provider_id: &str, model: &str, endpoint: &str) -> ResolvedModelConfig {
    ResolvedModelConfig {
        providerId: provider_id.into(),
        providerName: "OpenCode".into(),
        modelId: model.into(),
        apiKey: "test-key".into(),
        apiEndpoint: endpoint.into(),
        apiProviderType: ApiProviderType::OPENCODE,
        apiProviderTypeId: "OPENCODE".into(),
        useMultipleApiKeys: false,
        apiKeyPool: Vec::new(),
        currentKeyIndex: 0,
        keyRotationMode: "ROUND_ROBIN".into(),
        customHeaders:
            r#"{"user-agent":"wrong-agent","X-OPENCODE-SESSION":"wrong-session","X-Custom":"keep"}"#
                .into(),
        requestLimitPerMinute: 0,
        maxConcurrentRequests: 0,
        pricing: None,
        context: Default::default(),
        capabilities: Default::default(),
        builtinTools: Vec::new(),
        request: Default::default(),
        parameters: Vec::new(),
        thinking: Default::default(),
        thinkingLevel: None,
        summary: Default::default(),
        localRuntime: Default::default(),
    }
}

/// Checks the exact values and uniqueness of case-insensitive outbound header names.
fn assert_header(request: &HttpRequestData, name: &str, expected: &str) {
    let values: Vec<_> = request
        .headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(values, vec![expected], "{}: {name}", request.url);
}

/// Exercises factory identity propagation and every routed chat protocol at dispatch.
#[tokio::test]
async fn opencode_go_preserves_identity_headers_for_every_protocol() {
    let host = Arc::new(CapturingHost::default());
    setDefaultHttpHost(host.clone());
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(TestScheduler(std::time::Instant::now())));
    for stream in [false, true] {
        for endpoint in [
            "https://opencode.ai/zen/go",
            "https://proxy.test/zen/go/v1/",
        ] {
            for provider_id in ["profile-a", "profile-b"] {
                for (model, suffix, auth_header, auth) in [
                    (
                        "opencode-go/glm-5",
                        "/chat/completions",
                        "Authorization",
                        "Bearer test-key",
                    ),
                    ("gpt-5", "/responses", "Authorization", "Bearer test-key"),
                    ("minimax-m2.5", "/messages", "x-api-key", "test-key"),
                    (
                        "gemini-3-pro",
                        "/models/gemini-3-pro:generateContent",
                        "x-goog-api-key",
                        "test-key",
                    ),
                ] {
                    let spec = AIServiceFactory::create_service(ProviderCreateRequest {
                        config: config(provider_id, model, endpoint),
                        provider_type: ApiProviderType::OPENCODE,
                        provider_type_id: "OPENCODE".into(),
                        tool_pkg_provider_registered: false,
                    })
                    .unwrap();
                    let ProviderCreateParams::OpenCodeProvider {
                        api_endpoint,
                        provider_id,
                        model_name,
                        custom_headers,
                        protocol,
                        supports_vision,
                        supports_audio,
                        supports_video,
                        builtin_tools,
                        enable_tool_call,
                        ..
                    } = spec.params
                    else {
                        panic!("expected OpenCode provider")
                    };
                    let mut provider = OpenCodeProvider::new(
                        api_endpoint,
                        provider_id.clone(),
                        "test-key".into(),
                        model_name,
                        custom_headers.into_iter().collect(),
                        protocol,
                        supports_vision,
                        supports_audio,
                        supports_video,
                        builtin_tools,
                        enable_tool_call,
                        test_runtime_context(),
                    )
                    .unwrap();
                    let mut message =
                        send_request(vec![PromptTurn::new(PromptTurnKind::USER, "hi")]);
                    message.stream = stream;
                    match provider.send_message(message).await {
                        Ok(mut response) => {
                            response.collect(&mut |_| {}).await;
                            assert!(
                                response.terminal_failure().is_some(),
                                "fixture must reject the request"
                            );
                        }
                        Err(error) => {
                            assert!(error.to_string().contains("header fixture"), "{error}")
                        }
                    }
                    let request = host
                        .0
                        .lock()
                        .unwrap()
                        .pop()
                        .expect("provider must dispatch a request");
                    let suffix = if stream {
                        suffix.replace(":generateContent", ":streamGenerateContent?alt=sse")
                    } else {
                        suffix.to_string()
                    };
                    assert!(request.url.ends_with(&suffix), "{}", request.url);
                    assert_header(
                        &request,
                        "User-Agent",
                        concat!("Operit/", env!("CARGO_PKG_VERSION")),
                    );
                    assert_header(
                        &request,
                        "x-opencode-session",
                        &format!("operit-{provider_id}"),
                    );
                    assert_header(&request, "X-Custom", "keep");
                    assert_header(&request, auth_header, auth);
                }
            }
        }
    }
}
