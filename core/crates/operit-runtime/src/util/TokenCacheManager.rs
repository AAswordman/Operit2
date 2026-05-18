use crate::util::AppLogger::AppLogger;
use crate::util::ChatUtils::ChatUtils;

#[derive(Debug, Clone, Default)]
pub struct TokenCacheManager {
    previous_chat_history: Vec<(String, String)>,
    previous_history_token_count: usize,
    cached_input_token_count: usize,
    current_input_token_count: usize,
    output_token_count: usize,
}

impl TokenCacheManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cached_input_token_count(&self) -> usize {
        self.cached_input_token_count
    }

    pub fn current_input_token_count(&self) -> usize {
        self.current_input_token_count
    }

    pub fn total_input_token_count(&self) -> usize {
        self.cached_input_token_count + self.current_input_token_count
    }

    pub fn output_token_count(&self) -> usize {
        self.output_token_count
    }

    pub fn reset_token_counts(&mut self) {
        self.previous_chat_history.clear();
        self.previous_history_token_count = 0;
        self.cached_input_token_count = 0;
        self.current_input_token_count = 0;
        self.output_token_count = 0;
    }

    pub fn add_output_tokens(&mut self, tokens: usize) {
        self.output_token_count += tokens;
    }

    pub fn set_output_tokens(&mut self, tokens: usize) {
        self.output_token_count = tokens;
    }

    pub fn update_actual_tokens(&mut self, actual_input: usize, cached_input: usize) {
        self.current_input_token_count = actual_input;
        self.cached_input_token_count = cached_input;
    }

    pub fn calculate_input_tokens(
        &mut self,
        chat_history: &[(String, String)],
        tools_json: Option<&str>,
        update_state: bool,
    ) -> usize {
        let history_with_tools = history_with_tools(chat_history, tools_json);
        let common_prefix_length =
            find_common_prefix_length(&history_with_tools, &self.previous_chat_history);

        AppLogger::d(
            "TokenCacheManager",
            &format!(
                "history compare: current={}, previous={}, common={}",
                history_with_tools.len(),
                self.previous_chat_history.len(),
                common_prefix_length
            ),
        );

        let (cached_tokens, new_tokens) = if common_prefix_length > 0 {
            let cached_tokens = if common_prefix_length == self.previous_chat_history.len() {
                self.previous_history_token_count
            } else {
                calculate_tokens_for_history(&history_with_tools[..common_prefix_length])
            };
            let new_tokens = calculate_tokens_for_history(&history_with_tools[common_prefix_length..]);
            (cached_tokens, new_tokens)
        } else {
            (0, calculate_tokens_for_history(&history_with_tools))
        };

        if update_state {
            self.cached_input_token_count = cached_tokens;
            self.current_input_token_count = new_tokens;
            self.previous_history_token_count = cached_tokens + new_tokens;
            if !chat_history.is_empty() {
                self.previous_chat_history = history_with_tools;
            }
        }

        cached_tokens + new_tokens
    }
}

fn history_with_tools(
    chat_history: &[(String, String)],
    tools_json: Option<&str>,
) -> Vec<(String, String)> {
    let Some(tools) = tools_json.filter(|value| !value.is_empty()) else {
        return chat_history.to_vec();
    };
    let mut history = chat_history.to_vec();
    if let Some(index) = history.iter().position(|(role, _)| role == "system") {
        let (role, content) = &history[index];
        history[index] = (role.clone(), format!("{tools}\n{content}"));
    } else {
        history.insert(0, ("system".to_string(), tools.to_string()));
    }
    history
}

fn find_common_prefix_length(
    current: &[(String, String)],
    previous: &[(String, String)],
) -> usize {
    let mut common = 0;
    for (left, right) in current.iter().zip(previous.iter()) {
        if left == right {
            common += 1;
        } else {
            break;
        }
    }
    common
}

fn calculate_tokens_for_history(history: &[(String, String)]) -> usize {
    history
        .iter()
        .map(|(_, content)| ChatUtils::estimate_token_count(content))
        .sum()
}
