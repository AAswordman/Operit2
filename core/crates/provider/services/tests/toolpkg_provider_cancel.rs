//! Verifies the ToolPkg JS provider releases its service lock immediately and honours cancellation.

#![allow(non_snake_case, dead_code)]

use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{
    ApiProviderType, ModelCapabilities, ModelRequestSpec, ProviderProfile, ResolvedModelConfig,
};
use operit_providers::chat::llmprovider::AIService::{AIService, AiServiceError};
use operit_providers::chat::llmprovider::ApiKeyProvider::ApiKeyOutcome;
use operit_providers::chat::llmprovider::ToolPkgJsAiProviderService::ToolPkgJsAiProviderService;
use operit_providers::runtime_support::{
    ChatConfigurationRequest, ChatConfigurationResult, ProviderFunctionModelBinding,
    ProviderMessageTiming, ProviderPackageInfo, ProviderRuntimeContext, ProviderRuntimeSupport,
    ProviderToolPkgAiProviderRegistration,
};
use operit_providers::{chat, runtime_support};
use operit_util::stream::RevisableTextStream::ResponseStreamItem;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[path = "../../tests/ProviderMediaTestSupport.rs"]
mod fixtures;
#[path = "../../tests/ImmediateResponseScheduler.rs"]
mod scheduler;
use fixtures::send_request;

#[derive(Clone)]
enum HookBehavior {
    StreamThenHang,
    CompleteText(String),
    Fatal(String),
}

/// Runtime support whose package hook simulates a long-running plugin generation.
struct HookSupport {
    started: Arc<AtomicBool>,
    behavior: HookBehavior,
}

impl HookSupport {
    fn new(behavior: HookBehavior) -> Arc<Self> {
        Arc::new(Self {
            started: Arc::new(AtomicBool::new(false)),
            behavior,
        })
    }

    async fn wait_started(&self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.started.load(Ordering::SeqCst) {
            assert!(Instant::now() < deadline, "package hook must start executing");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}

impl ProviderRuntimeSupport for HookSupport {
    fn updateTokensForProviderModel(
        &self,
        _providerModel: &str,
        _inputTokens: i64,
        _outputTokens: i64,
        _cachedInputTokens: i64,
    ) -> Result<(), String> {
        Ok(())
    }

    fn disableUserPreferenceDescription(&self) -> Result<bool, String> {
        Ok(false)
    }

    fn defaultTtsConfigId(&self) -> Result<String, String> {
        Err("cancel fixture does not resolve speech configuration".to_string())
    }

    fn resolveChatConfiguration(
        &self,
        _request: ChatConfigurationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ChatConfigurationResult, String>> + Send + '_>,
    > {
        Box::pin(async { Err("cancel fixture does not invoke chat plugins".to_string()) })
    }

    fn aiVisibleSkillPackages(&self) -> Result<Vec<ProviderPackageInfo>, String> {
        Ok(Vec::new())
    }

    fn modelBindingForFunction(
        &self,
        _functionType: FunctionType,
    ) -> Result<ProviderFunctionModelBinding, String> {
        Err("cancel fixture does not expose model bindings".to_string())
    }

    fn resolvedModelConfig(
        &self,
        _providerId: &str,
        _modelId: &str,
    ) -> Result<ResolvedModelConfig, String> {
        Err("cancel fixture does not expose model configuration".to_string())
    }

    fn modelCapabilitiesForFunction(
        &self,
        _functionType: FunctionType,
    ) -> Result<Option<ModelCapabilities>, String> {
        Ok(None)
    }

    fn providerProfile(&self, _providerId: &str) -> Result<ProviderProfile, String> {
        Err("cancel fixture does not expose provider profiles".to_string())
    }

    fn reserveProviderApiKey(&self, _providerId: &str) -> Result<String, String> {
        Err("cancel fixture does not expose API key pools".to_string())
    }

    fn reportProviderApiKey(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: ApiKeyOutcome,
    ) -> Result<(), String> {
        Err("cancel fixture does not expose API key pools".to_string())
    }

    fn hasToolPkgAiProvider(&self, _providerId: &str) -> bool {
        true
    }

    fn toolPkgAiProvider(
        &self,
        _providerId: &str,
    ) -> Option<ProviderToolPkgAiProviderRegistration> {
        Some(registration())
    }

    fn runToolPkgAiProviderHook(
        &self,
        _containerPackageName: &str,
        _functionName: &str,
        _functionSource: Option<&str>,
        _event: &str,
        _tag: Option<String>,
        _sourceKey: Option<String>,
        _eventPayload: Value,
        _runtimeContextKey: Option<String>,
        _executionKind: Option<String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send>>
    {
        let behavior = self.behavior.clone();
        let started = self.started.clone();
        Box::pin(async move {
            match behavior {
                HookBehavior::StreamThenHang => {
                    if let Some(callback) = onIntermediateResult {
                        callback(r#"{"chunk":"hello"}"#.to_string());
                    }
                    started.store(true, Ordering::SeqCst);
                    std::future::pending::<()>().await;
                    Ok(None)
                }
                HookBehavior::CompleteText(text) => {
                    started.store(true, Ordering::SeqCst);
                    Ok(Some(json!({"success": true, "text": text}).to_string()))
                }
                HookBehavior::Fatal(message) => {
                    started.store(true, Ordering::SeqCst);
                    Ok(Some(json!({"success": false, "error": message}).to_string()))
                }
            }
        })
    }

    fn decodeToolPkgHookResult(&self, raw: Option<String>) -> Option<Value> {
        raw.and_then(|raw| serde_json::from_str(&raw).ok())
    }

    fn messageTimingNow(&self) -> ProviderMessageTiming {
        ProviderMessageTiming { startedAtMs: 0 }
    }

    fn logMessageTiming(
        &self,
        _stage: &str,
        _startTimeMs: ProviderMessageTiming,
        _details: Option<String>,
    ) {
    }
}

fn registration() -> ProviderToolPkgAiProviderRegistration {
    ProviderToolPkgAiProviderRegistration {
        containerPackageName: "com.example.provider".to_string(),
        providerId: "example".to_string(),
        displayName: "Example".to_string(),
        description: "Example plugin provider".to_string(),
        listModelsFunctionName: "listModels".to_string(),
        listModelsFunctionSource: None,
        sendMessageFunctionName: "sendMessage".to_string(),
        sendMessageFunctionSource: None,
        testConnectionFunctionName: "testConnection".to_string(),
        testConnectionFunctionSource: None,
        calculateInputTokensFunctionName: "calculateInputTokens".to_string(),
        calculateInputTokensFunctionSource: None,
    }
}

fn config() -> ResolvedModelConfig {
    ResolvedModelConfig {
        providerId: "example".into(),
        providerName: "Example".into(),
        modelId: "example-model".into(),
        apiKey: "test-key".into(),
        apiEndpoint: "https://example.test/v1/chat/completions".into(),
        apiProviderType: ApiProviderType::OPENAI,
        apiProviderTypeId: "OPENAI".into(),
        useMultipleApiKeys: false,
        apiKeyPool: Vec::new(),
        currentKeyIndex: 0,
        keyRotationMode: "ROUND_ROBIN".into(),
        customHeaders: "{}".into(),
        requestLimitPerMinute: 0,
        maxConcurrentRequests: 0,
        pricing: None,
        context: Default::default(),
        capabilities: Default::default(),
        builtinTools: Vec::new(),
        request: ModelRequestSpec {
            supportsStructuredTools: true,
            enableClaude1hPromptCache: false,
        },
        parameters: Vec::new(),
        thinking: Default::default(),
        thinkingLevel: None,
        summary: Default::default(),
        localRuntime: Default::default(),
    }
}

fn service(support: Arc<HookSupport>) -> ToolPkgJsAiProviderService {
    ToolPkgJsAiProviderService::new(
        config(),
        registration(),
        ProviderRuntimeContext::new(support),
        "test-key".into(),
    )
}

async fn collect_chunks(
    stream: &mut Box<dyn operit_util::stream::RevisableTextStream::RevisableTextStreamLike>,
) -> Vec<String> {
    let mut chunks = Vec::new();
    {
        let mut collector = |item: ResponseStreamItem| {
            if let ResponseStreamItem::Chunk(chunk) = item {
                chunks.push(chunk);
            }
        };
        let future = stream.collect_ordered(&mut collector);
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .expect("plugin response stream must close");
    }
    chunks
}

/// Cancelling an in-flight plugin request must close the stream instead of hanging until the hook finishes.
#[tokio::test]
async fn cancel_streaming_closes_inflight_plugin_stream() {
    scheduler::install();
    let support = HookSupport::new(HookBehavior::StreamThenHang);
    let mut provider = service(support.clone());

    let mut stream = provider
        .send_message(send_request(Vec::new()))
        .await
        .expect("send_message must return a stream while the hook is still running");
    support.wait_started().await;

    provider.cancel_streaming();
    let chunks = collect_chunks(&mut stream).await;

    assert!(chunks.contains(&"hello".to_string()));
    assert!(stream.terminal_failure().is_none());
}

/// A hook that completes without intermediate chunks must still publish its final text.
#[tokio::test]
async fn completed_hook_publishes_final_text() {
    scheduler::install();
    let support = HookSupport::new(HookBehavior::CompleteText("final answer".to_string()));
    let mut provider = service(support.clone());

    let mut stream = provider
        .send_message(send_request(Vec::new()))
        .await
        .expect("send_message must return a stream");
    let chunks = collect_chunks(&mut stream).await;

    assert_eq!(chunks, vec!["final answer".to_string()]);
    assert!(stream.terminal_failure().is_none());
}

/// A fatal hook failure must surface through the stream terminal failure once the hook runs in background.
#[tokio::test]
async fn fatal_hook_failure_surfaces_as_terminal_failure() {
    scheduler::install();
    let support = HookSupport::new(HookBehavior::Fatal("boom".to_string()));
    let mut provider = service(support.clone());

    let mut stream = provider
        .send_message(send_request(Vec::new()))
        .await
        .expect("send_message must return a stream");
    let _chunks = collect_chunks(&mut stream).await;

    assert_eq!(
        stream.terminal_failure(),
        Some(AiServiceError::RequestFailed("boom".to_string()).to_string())
    );
}
