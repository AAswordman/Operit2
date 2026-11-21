use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};

#[derive(Clone)]
pub struct TokenStatisticsDelegate {
    pub cumulativeInputTokens: i64,
    pub cumulativeOutputTokens: i64,
    pub currentWindowSize: i64,
    pub perRequestTokenCount: Option<(i64, i64)>,
    pub cumulativeInputTokensFlow: MutableStateFlow<i64>,
    pub cumulativeOutputTokensFlow: MutableStateFlow<i64>,
    pub currentWindowSizeFlow: MutableStateFlow<i64>,
    pub perRequestTokenCountFlow: MutableStateFlow<Option<(i64, i64)>>,
    pub lastCurrentWindowSize: i64,
    pub cumulativeInputTokensByChatKey: HashMap<String, i64>,
    pub cumulativeOutputTokensByChatKey: HashMap<String, i64>,
    // Sends and summaries run on deep-cloned workers; estimates must reach the owning Core.
    pub lastWindowSizeByChatKey: Arc<Mutex<HashMap<String, i64>>>,
    pub perRequestTokenCountByChatKey: HashMap<String, Option<(i64, i64)>>,
    pub activeChatId: Option<String>,
}

impl TokenStatisticsDelegate {
    pub fn new() -> Self {
        Self {
            cumulativeInputTokens: 0,
            cumulativeOutputTokens: 0,
            currentWindowSize: 0,
            perRequestTokenCount: None,
            cumulativeInputTokensFlow: mutableStateFlow(0),
            cumulativeOutputTokensFlow: mutableStateFlow(0),
            currentWindowSizeFlow: mutableStateFlow(0),
            perRequestTokenCountFlow: mutableStateFlow(None),
            lastCurrentWindowSize: 0,
            cumulativeInputTokensByChatKey: HashMap::new(),
            cumulativeOutputTokensByChatKey: HashMap::new(),
            lastWindowSizeByChatKey: Arc::new(Mutex::new(HashMap::new())),
            perRequestTokenCountByChatKey: HashMap::new(),
            activeChatId: None,
        }
    }

    fn chatKey(chatId: Option<&String>) -> String {
        chatId
            .cloned()
            .unwrap_or_else(|| "__DEFAULT_CHAT__".to_string())
    }

    fn isActiveKey(&self, key: &str) -> bool {
        key == Self::chatKey(self.activeChatId.as_ref())
    }

    fn refreshActiveFromCache(&mut self) {
        let key = Self::chatKey(self.activeChatId.as_ref());
        let input = self
            .cumulativeInputTokensByChatKey
            .get(&key)
            .copied()
            .unwrap_or(0);
        let output = self
            .cumulativeOutputTokensByChatKey
            .get(&key)
            .copied()
            .unwrap_or(0);
        let window = self.windowCache().get(&key).copied().unwrap_or(0);
        let perRequest = self
            .perRequestTokenCountByChatKey
            .get(&key)
            .cloned()
            .flatten();
        self.cumulativeInputTokens = input;
        self.cumulativeOutputTokens = output;
        self.currentWindowSize = window;
        self.perRequestTokenCount = perRequest;
        self.cumulativeInputTokensFlow.set_value(input);
        self.cumulativeOutputTokensFlow.set_value(output);
        self.currentWindowSizeFlow.set_value(window);
        self.perRequestTokenCountFlow.set_value(perRequest);
        self.lastCurrentWindowSize = window;
    }

    #[allow(non_snake_case)]
    pub fn setupCollectors(&mut self) {}

    #[allow(non_snake_case)]
    pub fn setActiveChatId(&mut self, chatId: Option<String>) {
        self.activeChatId = chatId;
        self.refreshActiveFromCache();
    }

    #[allow(non_snake_case)]
    pub fn bindChatService(&mut self, chatId: Option<String>, service: &EnhancedAIService) {
        let key = Self::chatKey(chatId.as_ref());
        self.handlePerRequestCounts(key.clone(), service.getPerRequestTokenCounts());
        self.handleRequestWindowEstimate(key.clone(), service.getRequestWindowEstimate());
        let mut delegate = self.clone();
        let flow = service.requestWindowEstimateFlow();
        flow.subscribe(move |windowSize| {
            delegate.handleRequestWindowEstimate(key.clone(), windowSize);
        });
    }

    #[allow(non_snake_case)]
    fn handlePerRequestCounts(&mut self, key: String, counts: Option<(i64, i64)>) {
        if counts.is_some() {
            self.perRequestTokenCountByChatKey
                .insert(key.clone(), counts);
        } else {
            self.perRequestTokenCountByChatKey.remove(&key);
        }
        if self.isActiveKey(&key) {
            self.perRequestTokenCount = counts;
        }
    }

    #[allow(non_snake_case)]
    fn handleRequestWindowEstimate(&mut self, key: String, windowSize: Option<i64>) {
        if let Some(windowSize) = windowSize {
            self.windowCache().insert(key.clone(), windowSize);
            if self.isActiveKey(&key) {
                self.currentWindowSize = windowSize;
                self.currentWindowSizeFlow.set_value(windowSize);
                self.lastCurrentWindowSize = windowSize;
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn resetTokenStatistics(&mut self, service: Option<&mut EnhancedAIService>) {
        self.cumulativeInputTokens = 0;
        self.cumulativeOutputTokens = 0;
        self.currentWindowSize = 0;
        self.perRequestTokenCount = None;
        self.cumulativeInputTokensFlow.set_value(0);
        self.cumulativeOutputTokensFlow.set_value(0);
        self.currentWindowSizeFlow.set_value(0);
        self.perRequestTokenCountFlow.set_value(None);
        self.lastCurrentWindowSize = 0;
        self.cumulativeInputTokensByChatKey.clear();
        self.cumulativeOutputTokensByChatKey.clear();
        self.windowCache().clear();
        self.perRequestTokenCountByChatKey.clear();
        if let Some(service) = service {
            service.resetTokenCounters();
        }
    }

    #[allow(non_snake_case)]
    pub fn updateCumulativeStatistics(
        &mut self,
        chatId: Option<String>,
        serviceOverride: Option<&EnhancedAIService>,
    ) {
        let key = Self::chatKey(chatId.as_ref().or(self.activeChatId.as_ref()));
        if let Some(service) = serviceOverride {
            let currentInputTokens = service.getCurrentInputTokenCount();
            let currentOutputTokens = service.getCurrentOutputTokenCount();
            let newInput = self
                .cumulativeInputTokensByChatKey
                .get(&key)
                .copied()
                .unwrap_or(0)
                + currentInputTokens;
            let newOutput = self
                .cumulativeOutputTokensByChatKey
                .get(&key)
                .copied()
                .unwrap_or(0)
                + currentOutputTokens;
            self.cumulativeInputTokensByChatKey
                .insert(key.clone(), newInput);
            self.cumulativeOutputTokensByChatKey
                .insert(key.clone(), newOutput);
            if self.isActiveKey(&key) {
                self.cumulativeInputTokens = newInput;
                self.cumulativeOutputTokens = newOutput;
                self.cumulativeInputTokensFlow.set_value(newInput);
                self.cumulativeOutputTokensFlow.set_value(newOutput);
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn setTokenCounts(
        &mut self,
        chatId: Option<String>,
        inputTokens: i64,
        outputTokens: i64,
        windowSize: i64,
    ) {
        let key = Self::chatKey(chatId.as_ref());
        self.cumulativeInputTokensByChatKey
            .insert(key.clone(), inputTokens);
        self.cumulativeOutputTokensByChatKey
            .insert(key.clone(), outputTokens);
        self.windowCache().insert(key.clone(), windowSize);
        if self.isActiveKey(&key) {
            self.cumulativeInputTokens = inputTokens;
            self.cumulativeOutputTokens = outputTokens;
            self.currentWindowSize = windowSize;
            self.cumulativeInputTokensFlow.set_value(inputTokens);
            self.cumulativeOutputTokensFlow.set_value(outputTokens);
            self.currentWindowSizeFlow.set_value(windowSize);
            self.lastCurrentWindowSize = windowSize;
        }
    }

    #[allow(non_snake_case)]
    pub fn getCumulativeTokenCounts(&self, chatId: Option<String>) -> (i64, i64) {
        let key = Self::chatKey(chatId.as_ref().or(self.activeChatId.as_ref()));
        (
            self.cumulativeInputTokensByChatKey
                .get(&key)
                .copied()
                .unwrap_or(0),
            self.cumulativeOutputTokensByChatKey
                .get(&key)
                .copied()
                .unwrap_or(0),
        )
    }

    #[allow(non_snake_case)]
    pub fn getLastCurrentWindowSize(&self, chatId: Option<String>) -> i64 {
        let key = Self::chatKey(chatId.as_ref().or(self.activeChatId.as_ref()));
        self.windowCache().get(&key).copied().unwrap_or(0)
    }

    fn windowCache(&self) -> std::sync::MutexGuard<'_, HashMap<String, i64>> {
        self.lastWindowSizeByChatKey
            .lock()
            .expect("token statistics window cache mutex poisoned")
    }

    #[allow(non_snake_case)]
    pub fn currentWindowSizeFlow(&self) -> StateFlow<i64> {
        self.currentWindowSizeFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn cumulativeInputTokensFlow(&self) -> StateFlow<i64> {
        self.cumulativeInputTokensFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn cumulativeOutputTokensFlow(&self) -> StateFlow<i64> {
        self.cumulativeOutputTokensFlow.asStateFlow()
    }

    #[allow(non_snake_case)]
    pub fn perRequestTokenCountFlow(&self) -> StateFlow<Option<(i64, i64)>> {
        self.perRequestTokenCountFlow.asStateFlow()
    }
}

impl Default for TokenStatisticsDelegate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod window_cache_sharing_tests {
    use super::*;

    #[test]
    fn worker_clone_estimates_reach_the_owning_delegate() {
        let owner = TokenStatisticsDelegate::new();
        let mut worker = owner.clone();
        worker.handleRequestWindowEstimate("chat-a".into(), Some(5000));
        assert_eq!(owner.getLastCurrentWindowSize(Some("chat-a".into())), 5000);

        let mut compressionWorker = owner.clone();
        compressionWorker.setTokenCounts(Some("chat-a".into()), 10, 20, 800);
        assert_eq!(owner.getLastCurrentWindowSize(Some("chat-a".into())), 800);

        let mut resetter = owner.clone();
        resetter.resetTokenStatistics(None);
        assert_eq!(owner.getLastCurrentWindowSize(Some("chat-a".into())), 0);
    }
}
