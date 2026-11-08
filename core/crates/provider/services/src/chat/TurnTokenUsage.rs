use std::sync::{Arc, Mutex};

use super::enhance::MultiServiceManager::SharedAIServiceHandle;
use super::llmprovider::AIService::{AiServiceError, TokenCounts};
use crate::runtime_support::ProviderRuntimeSupport;
use operit_model::FunctionType::FunctionType;
use operit_store::repository::UsageStatisticsStore::{UsageRequestSource, UsageStatisticsStore};

/// Token totals owned by one logical turn, including requests interrupted by cancellation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnTokenSnapshot {
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub cachedInputTokens: i64,
}

/// Shares one turn's accounting with its producer and the message persistence owner.
#[derive(Clone, Debug, Default)]
pub struct TurnTokenUsage(Arc<Mutex<TurnTokenUsageState>>);

#[derive(Debug, Default)]
struct TurnTokenUsageState {
    snapshot: TurnTokenSnapshot,
    lastRequestTokens: Option<(i64, i64)>,
    request: Option<Arc<RequestTokenUsage>>,
}

/// Serializes settlement of a physical request across completion and cancellation.
pub(super) struct RequestTokenUsage {
    service: SharedAIServiceHandle,
    functionType: FunctionType,
    source: UsageRequestSource,
    chatId: Option<String>,
    result: tokio::sync::Mutex<Option<Result<TokenCounts, AiServiceError>>>,
}

impl std::fmt::Debug for RequestTokenUsage {
    /// Prints request identity without requiring the provider service to implement Debug.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestTokenUsage")
            .field("source", &self.source)
            .field("chatId", &self.chatId)
            .finish_non_exhaustive()
    }
}

impl TurnTokenUsage {
    /// Returns this turn's settled totals, independently of any newer turn.
    pub fn snapshot(&self) -> TurnTokenSnapshot {
        self.0
            .lock()
            .expect("turn token usage mutex poisoned")
            .snapshot
            .clone()
    }

    /// Sets explicit totals when resuming an already-accounted turn.
    pub(super) fn setSnapshot(&self, snapshot: TurnTokenSnapshot) {
        let mut state = self.0.lock().expect("turn token usage mutex poisoned");
        state.lastRequestTokens = Some((snapshot.inputTokens, snapshot.outputTokens));
        state.snapshot = snapshot;
    }

    /// Clears displayed totals without detaching an active request from its turn owner.
    pub(super) fn reset(&self) {
        let mut state = self.0.lock().expect("turn token usage mutex poisoned");
        state.snapshot = TurnTokenSnapshot::default();
        state.lastRequestTokens = None;
    }

    /// Returns the latest settled physical request's input and output counters.
    pub(super) fn perRequestTokens(&self) -> Option<(i64, i64)> {
        self.0
            .lock()
            .expect("turn token usage mutex poisoned")
            .lastRequestTokens
    }

    /// Registers a request while its provider handle is locked and its execution is active.
    pub(super) fn beginRequest(
        &self,
        service: SharedAIServiceHandle,
        functionType: FunctionType,
        source: UsageRequestSource,
        chatId: Option<String>,
    ) -> Arc<RequestTokenUsage> {
        let request = Arc::new(RequestTokenUsage {
            service,
            functionType,
            source,
            chatId,
            result: tokio::sync::Mutex::new(None),
        });
        let mut state = self.0.lock().expect("turn token usage mutex poisoned");
        state.lastRequestTokens = None;
        state.request = Some(request.clone());
        request
    }

    /// Settles the current request before cancellation hands its totals to persistence.
    pub(super) async fn finishCurrentRequest(
        &self,
        runtime: &dyn ProviderRuntimeSupport,
    ) -> Result<(), AiServiceError> {
        let request = self
            .0
            .lock()
            .expect("turn token usage mutex poisoned")
            .request
            .clone();
        if let Some(request) = request {
            self.finishRequest(&request, runtime).await?;
        }
        Ok(())
    }

    /// Records one physical request once, even when cancellation races its collector.
    pub(super) async fn finishRequest(
        &self,
        request: &RequestTokenUsage,
        runtime: &dyn ProviderRuntimeSupport,
    ) -> Result<TokenCounts, AiServiceError> {
        self.finishRequestWith(request, |providerModel, tokens| {
            runtime
                .updateTokensForProviderModel(
                    providerModel,
                    tokens.input,
                    tokens.output,
                    tokens.cached_input,
                )
                .map_err(AiServiceError::RequestFailed)?;
            UsageStatisticsStore::new()
                .recordProviderModelRequest(
                    providerModel.to_string(),
                    request.functionType.clone(),
                    request.source.clone(),
                    request.chatId.clone(),
                    tokens.input,
                    tokens.output,
                    tokens.cached_input,
                )
                .map(|_| ())
                .map_err(AiServiceError::RequestFailed)
        })
        .await
    }

    /// Captures provider counters and caches both successful and failed settlement results.
    async fn finishRequestWith(
        &self,
        request: &RequestTokenUsage,
        record: impl FnOnce(&str, &TokenCounts) -> Result<(), AiServiceError>,
    ) -> Result<TokenCounts, AiServiceError> {
        let mut result = request.result.lock().await;
        if let Some(result) = result.as_ref() {
            return result.clone();
        }
        let service = request.service.lock().await;
        let tokens = TokenCounts {
            input: service.input_token_count().max(0),
            output: service.output_token_count().max(0),
            cached_input: service.cached_input_token_count().max(0),
        };
        let providerModel = service.provider_model();
        drop(service);
        {
            let mut state = self.0.lock().expect("turn token usage mutex poisoned");
            state.snapshot.inputTokens += tokens.input;
            state.snapshot.outputTokens += tokens.output;
            state.snapshot.cachedInputTokens += tokens.cached_input;
            state.lastRequestTokens = Some((tokens.input, tokens.output));
        }
        let settled = record(&providerModel, &tokens).map(|()| tokens);
        *result = Some(settled.clone());
        settled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::llmprovider::AIService::AIService;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CounterService(Arc<Mutex<TokenCounts>>);

    impl AIService for CounterService {
        /// Returns the current request's input count.
        fn input_token_count(&self) -> i64 {
            self.0.lock().unwrap().input
        }
        /// Returns the current request's cache count.
        fn cached_input_token_count(&self) -> i64 {
            self.0.lock().unwrap().cached_input
        }
        /// Returns the current request's output count.
        fn output_token_count(&self) -> i64 {
            self.0.lock().unwrap().output
        }
        /// Identifies the request's provider for accounting assertions.
        fn provider_model(&self) -> String {
            "TEST:model".into()
        }
    }

    /// Builds independently mutable provider counters without network or storage dependencies.
    fn counterService(
        input: i64,
        output: i64,
        cached: i64,
    ) -> (SharedAIServiceHandle, Arc<Mutex<TokenCounts>>) {
        let counters = Arc::new(Mutex::new(TokenCounts {
            input,
            output,
            cached_input: cached,
        }));
        let service: SharedAIServiceHandle = Arc::new(tokio::sync::Mutex::new(Box::new(
            CounterService(counters.clone()),
        )));
        (service, counters)
    }

    /// Records interrupted first-request usage exactly once when both terminal owners settle it.
    #[tokio::test]
    async fn cancellation_racing_completion_records_request_once() {
        let usage = TurnTokenUsage::default();
        let (service, _) = counterService(120, 35, 40);
        let request = usage.beginRequest(
            service,
            FunctionType::CHAT,
            UsageRequestSource::CHAT_RESPONSE,
            Some("chat-a".into()),
        );
        let writes = AtomicUsize::new(0);
        let providerLock = request.service.lock().await;
        let record = |provider: &str, tokens: &TokenCounts| {
            assert_eq!(provider, "TEST:model");
            assert_eq!(
                (tokens.input, tokens.output, tokens.cached_input),
                (120, 35, 40)
            );
            writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        let (cancelled, completed, _) = tokio::join!(
            usage.finishRequestWith(&request, record),
            usage.finishRequestWith(&request, record),
            async {
                tokio::task::yield_now().await;
                drop(providerLock);
            },
        );
        assert_eq!(cancelled.unwrap(), completed.unwrap());
        assert_eq!(writes.load(Ordering::SeqCst), 1);
        assert_eq!(
            usage.snapshot(),
            TurnTokenSnapshot {
                inputTokens: 120,
                outputTokens: 35,
                cachedInputTokens: 40
            }
        );
    }

    /// Includes previous tool rounds and the interrupted current request without recounting old usage.
    #[tokio::test]
    async fn interrupted_tool_round_accumulates_all_requests_once() {
        let usage = TurnTokenUsage::default();
        let (service, counters) = counterService(100, 20, 30);
        let first = usage.beginRequest(
            service.clone(),
            FunctionType::CHAT,
            UsageRequestSource::CHAT_RESPONSE,
            None,
        );
        usage
            .finishRequestWith(&first, |_, _| Ok(()))
            .await
            .unwrap();
        *counters.lock().unwrap() = TokenCounts {
            input: 150,
            output: 12,
            cached_input: 80,
        };
        let second = usage.beginRequest(
            service,
            FunctionType::CHAT,
            UsageRequestSource::TOOL_RESULT_RESPONSE,
            None,
        );
        usage
            .finishRequestWith(&second, |_, _| Ok(()))
            .await
            .unwrap();
        usage
            .finishRequestWith(&first, |_, _| {
                panic!("settled request must not be recorded again")
            })
            .await
            .unwrap();
        usage
            .finishRequestWith(&second, |_, _| {
                panic!("pausing during tools must not recount the last request")
            })
            .await
            .unwrap();
        assert_eq!(
            usage.snapshot(),
            TurnTokenSnapshot {
                inputTokens: 250,
                outputTokens: 32,
                cachedInputTokens: 110
            }
        );
        assert_eq!(usage.perRequestTokens(), Some((150, 12)));
    }

    /// Keeps the original turn stable when a newer turn reuses the provider's mutable counters.
    #[tokio::test]
    async fn late_completion_cannot_modify_a_new_turn() {
        let old = TurnTokenUsage::default();
        let new = TurnTokenUsage::default();
        let (service, counters) = counterService(10, 5, 2);
        let first = old.beginRequest(
            service.clone(),
            FunctionType::CHAT,
            UsageRequestSource::CHAT_RESPONSE,
            None,
        );
        old.finishRequestWith(&first, |_, _| Ok(())).await.unwrap();
        *counters.lock().unwrap() = TokenCounts {
            input: 200,
            output: 50,
            cached_input: 20,
        };
        let second = new.beginRequest(
            service,
            FunctionType::CHAT,
            UsageRequestSource::CHAT_RESPONSE,
            None,
        );
        new.finishRequestWith(&second, |_, _| Ok(())).await.unwrap();
        old.finishRequestWith(&first, |_, _| {
            panic!("late finalizer must use the recorded result")
        })
        .await
        .unwrap();
        assert_eq!(old.snapshot().inputTokens, 10);
        assert_eq!(new.snapshot().inputTokens, 200);
    }

    /// Surfaces storage failure while retaining actual message usage and avoiding repeated side effects.
    #[tokio::test]
    async fn failed_persistence_is_reported_without_recounting() {
        let usage = TurnTokenUsage::default();
        let (service, _) = counterService(20, 7, 3);
        let request = usage.beginRequest(
            service,
            FunctionType::CHAT,
            UsageRequestSource::CHAT_RESPONSE,
            None,
        );
        let error = AiServiceError::RequestFailed("storage unavailable".into());
        assert_eq!(
            usage
                .finishRequestWith(&request, |_, _| Err(error.clone()))
                .await
                .unwrap_err(),
            error
        );
        assert_eq!(
            usage
                .finishRequestWith(&request, |_, _| panic!(
                    "a failed write must not be retried implicitly"
                ))
                .await
                .unwrap_err(),
            error
        );
        assert_eq!(usage.snapshot().outputTokens, 7);
    }

    /// Starts an unsent turn at zero without inheriting another turn's counters.
    #[test]
    fn unsent_turn_has_no_recorded_request() {
        let usage = TurnTokenUsage::default();
        assert_eq!(usage.snapshot(), TurnTokenSnapshot::default());
        assert_eq!(usage.perRequestTokens(), None);
    }
}
