#[path = "../../tests/ApiKeyPoolTestSupport.rs"]
mod support;
#[path = "../../tests/ImmediateResponseScheduler.rs"]
mod scheduler;

use operit_model::ModelConfigData::{ApiProviderType, ProviderProfile};
use operit_providers::chat::llmprovider::ApiKeyProvider::{ApiKeyPoolState, KeyGetter};
use support::{fixture, ScriptHost};

/// Uses the identical decision policy for actual cached chat requests and synchronous model discovery.
#[tokio::test]
async fn key_pool_cached_chat_and_discovery_share_affinity() {
    use operit_providers::chat::llmprovider::AIService::{AIService, SendMessageRequest};
    use operit_providers::chat::llmprovider::ClaudeProvider::ClaudeProvider;
    use operit_providers::chat::llmprovider::ThinkingDialect::RequestThinking;
    use operit_util::stream::Stream::Stream as TextStream;
    scheduler::install();
    let (session, _) = fixture();
    let host = ScriptHost::new(&[
        (200, false),
        (401, false),
        (200, false),
        (200, false),
        (200, false),
    ]);
    operit_host_api::HostManager::setDefaultHttpHost(host.clone());
    let mut service = ClaudeProvider::new(
        "https://fixture.invalid/v1/messages".into(),
        session.clone(),
        "model".into(),
        "ANTHROPIC_GENERIC".into(),
        Vec::new(),
        false,
    );
    for _ in 0..3 {
        let mut response = service
            .send_message(SendMessageRequest {
                chat_history: Vec::new(),
                model_parameters: Vec::new(),
                thinking: RequestThinking::default(),
                stream: false,
                available_tools: Vec::new(),
                preserve_think_in_history: false,
                enable_retry: false,
                on_non_fatal_error: None,
                on_tool_invocation: None,
            })
            .await
            .unwrap();
        response.collect(&mut |_| {}).await;
        assert!(response.terminal_failure().is_none());
    }
    let mut provider = ProviderProfile::new(
        "pool".into(),
        "Pool".into(),
        ApiProviderType::ANTHROPIC_GENERIC,
        "https://fixture.invalid/v1".into(),
    );
    provider.apiKey = "unused".into();
    let catalog = operit_model::ModelCatalog::ModelCatalog::provider("ANTHROPIC_GENERIC").unwrap();
    operit_providers::chat::llmprovider::ModelListFetcher::ModelListFetcher::fetch(
        &provider, &catalog, session,
    )
    .unwrap();
    assert_eq!(
        host.header_values("x-api-key"),
        ["key-0", "key-0", "key-1", "key-1", "key-1"]
    );
}
