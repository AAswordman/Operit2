use std::collections::HashMap;

use crate::api::chat::EnhancedAIService::EnhancedAIService;

#[derive(Clone, Debug)]
pub struct TokenStatisticsDelegate {
    pub cumulativeInputTokens: i32,
    pub cumulativeOutputTokens: i32,
    pub currentWindowSize: i32,
    pub perRequestTokenCount: Option<(i32, i32)>,
    pub lastCurrentWindowSize: i32,
    pub cumulativeInputTokensByChatKey: HashMap<String, i32>,
    pub cumulativeOutputTokensByChatKey: HashMap<String, i32>,
    pub lastWindowSizeByChatKey: HashMap<String, i32>,
    pub perRequestTokenCountByChatKey: HashMap<String, Option<(i32, i32)>>,
    pub activeChatId: Option<String>,
}

impl TokenStatisticsDelegate {
    pub fn new() -> Self {
        Self {
            cumulativeInputTokens: 0,
            cumulativeOutputTokens: 0,
            currentWindowSize: 0,
            perRequestTokenCount: None,
            lastCurrentWindowSize: 0,
            cumulativeInputTokensByChatKey: HashMap::new(),
            cumulativeOutputTokensByChatKey: HashMap::new(),
            lastWindowSizeByChatKey: HashMap::new(),
            perRequestTokenCountByChatKey: HashMap::new(),
            activeChatId: None,
        }
    }

    fn chatKey(chatId: Option<&String>) -> String {
        chatId.cloned().unwrap_or_else(|| "__DEFAULT_CHAT__".to_string())
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
        let window = self.lastWindowSizeByChatKey.get(&key).copied().unwrap_or(0);
        let perRequest = self.perRequestTokenCountByChatKey.get(&key).cloned().flatten();
        self.cumulativeInputTokens = input;
        self.cumulativeOutputTokens = output;
        self.currentWindowSize = window;
        self.perRequestTokenCount = perRequest;
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
        self.handlePerRequestCounts(key.clone(), service.per_request_token_counts);
        self.handleRequestWindowEstimate(key);
    }

    #[allow(non_snake_case)]
    fn handlePerRequestCounts(&mut self, key: String, counts: Option<(i32, i32)>) {
        if counts.is_some() {
            self.perRequestTokenCountByChatKey.insert(key.clone(), counts);
        } else {
            self.perRequestTokenCountByChatKey.remove(&key);
        }
        if self.isActiveKey(&key) {
            self.perRequestTokenCount = counts;
        }
    }

    #[allow(non_snake_case)]
    fn handleRequestWindowEstimate(&mut self, key: String) {
        let windowSize = self.lastWindowSizeByChatKey.get(&key).copied();
        if let Some(windowSize) = windowSize {
            self.lastWindowSizeByChatKey.insert(key.clone(), windowSize);
            if self.isActiveKey(&key) {
                self.currentWindowSize = windowSize;
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
        self.lastCurrentWindowSize = 0;
        self.cumulativeInputTokensByChatKey.clear();
        self.cumulativeOutputTokensByChatKey.clear();
        self.lastWindowSizeByChatKey.clear();
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
            self.cumulativeInputTokensByChatKey.insert(key.clone(), newInput);
            self.cumulativeOutputTokensByChatKey.insert(key.clone(), newOutput);
            if self.isActiveKey(&key) {
                self.cumulativeInputTokens = newInput;
                self.cumulativeOutputTokens = newOutput;
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn setTokenCounts(
        &mut self,
        chatId: Option<String>,
        inputTokens: i32,
        outputTokens: i32,
        windowSize: i32,
    ) {
        let key = Self::chatKey(chatId.as_ref());
        self.cumulativeInputTokensByChatKey
            .insert(key.clone(), inputTokens);
        self.cumulativeOutputTokensByChatKey
            .insert(key.clone(), outputTokens);
        self.lastWindowSizeByChatKey.insert(key.clone(), windowSize);
        if self.isActiveKey(&key) {
            self.cumulativeInputTokens = inputTokens;
            self.cumulativeOutputTokens = outputTokens;
            self.currentWindowSize = windowSize;
            self.lastCurrentWindowSize = windowSize;
        }
    }

    #[allow(non_snake_case)]
    pub fn getCumulativeTokenCounts(&self, chatId: Option<String>) -> (i32, i32) {
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
    pub fn getLastCurrentWindowSize(&self, chatId: Option<String>) -> i32 {
        let key = Self::chatKey(chatId.as_ref().or(self.activeChatId.as_ref()));
        self.lastWindowSizeByChatKey.get(&key).copied().unwrap_or(0)
    }
}

impl Default for TokenStatisticsDelegate {
    fn default() -> Self {
        Self::new()
    }
}
