use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use super::PromptTurn::PromptTurn;

#[derive(Clone, Debug)]
pub struct SummaryHookContext {
    pub stage: String,
    pub use_english: Option<bool>,
    pub previous_summary: Option<String>,
    pub chat_history: Vec<PromptTurn>,
    pub prepared_history: Vec<PromptTurn>,
    pub system_prompt: Option<String>,
    pub summary_prompt: Option<String>,
    pub summary_result: Option<String>,
    pub model_parameters: Vec<HashMap<String, Value>>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Clone, Debug, Default)]
pub struct SummaryHookMutation {
    pub chat_history: Option<Vec<PromptTurn>>,
    pub prepared_history: Option<Vec<PromptTurn>>,
    pub system_prompt: Option<String>,
    pub summary_prompt: Option<String>,
    pub summary_result: Option<String>,
    pub metadata: HashMap<String, Value>,
}

pub trait SummaryGenerateHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &SummaryHookContext) -> Option<SummaryHookMutation> {
        None
    }
}

type SummaryGenerateHookList = Mutex<Vec<Arc<dyn SummaryGenerateHook>>>;
static SUMMARY_GENERATE_HOOKS: OnceLock<SummaryGenerateHookList> = OnceLock::new();

pub struct SummaryHookRegistry;

impl SummaryHookRegistry {
    #[allow(non_snake_case)]
    pub fn registerSummaryGenerateHook(hook: Arc<dyn SummaryGenerateHook>) {
        Self::unregisterSummaryGenerateHook(hook.id());
        SUMMARY_GENERATE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap()
            .push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterSummaryGenerateHook(hook_id: &str) {
        SUMMARY_GENERATE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap()
            .retain(|hook| hook.id() != hook_id);
    }

    #[allow(non_snake_case)]
    pub fn dispatchSummaryGenerateHooks(initial_context: SummaryHookContext) -> SummaryHookContext {
        let snapshot = SUMMARY_GENERATE_HOOKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap()
            .clone();
        let mut current = initial_context;
        for hook in snapshot {
            if let Some(mutation) = hook.on_event(&current) {
                current = apply_mutation(current, mutation);
            }
        }
        current
    }
}

fn apply_mutation(mut current: SummaryHookContext, mutation: SummaryHookMutation) -> SummaryHookContext {
    if let Some(chat_history) = mutation.chat_history {
        current.chat_history = chat_history;
    }
    if let Some(prepared_history) = mutation.prepared_history {
        current.prepared_history = prepared_history;
    }
    if let Some(system_prompt) = mutation.system_prompt {
        current.system_prompt = Some(system_prompt);
    }
    if let Some(summary_prompt) = mutation.summary_prompt {
        current.summary_prompt = Some(summary_prompt);
    }
    if let Some(summary_result) = mutation.summary_result {
        current.summary_result = Some(summary_result);
    }
    if !mutation.metadata.is_empty() {
        current.metadata.extend(mutation.metadata);
    }
    current
}
