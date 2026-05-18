use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use super::PromptTurn::PromptTurn;

#[derive(Clone, Debug)]
pub struct PromptHookContext {
    pub stage: String,
    pub chat_id: Option<String>,
    pub function_type: Option<String>,
    pub prompt_function_type: Option<String>,
    pub use_english: Option<bool>,
    pub raw_input: Option<String>,
    pub processed_input: Option<String>,
    pub chat_history: Vec<PromptTurn>,
    pub prepared_history: Vec<PromptTurn>,
    pub system_prompt: Option<String>,
    pub tool_prompt: Option<String>,
    pub model_parameters: Vec<HashMap<String, Value>>,
    pub available_tools: Vec<HashMap<String, Value>>,
    pub metadata: HashMap<String, Value>,
}

impl Default for PromptHookContext {
    fn default() -> Self {
        Self {
            stage: String::new(),
            chat_id: None,
            function_type: None,
            prompt_function_type: None,
            use_english: None,
            raw_input: None,
            processed_input: None,
            chat_history: Vec::new(),
            prepared_history: Vec::new(),
            system_prompt: None,
            tool_prompt: None,
            model_parameters: Vec::new(),
            available_tools: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PromptHookMutation {
    pub raw_input: Option<String>,
    pub processed_input: Option<String>,
    pub chat_history: Option<Vec<PromptTurn>>,
    pub prepared_history: Option<Vec<PromptTurn>>,
    pub system_prompt: Option<String>,
    pub tool_prompt: Option<String>,
    pub available_tools: Option<Vec<HashMap<String, Value>>>,
    pub metadata: HashMap<String, Value>,
}

pub trait PromptInputHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &PromptHookContext) -> Option<PromptHookMutation> {
        None
    }
}

pub trait PromptHistoryHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &PromptHookContext) -> Option<PromptHookMutation> {
        None
    }
}

pub trait PromptEstimateHistoryHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &PromptHookContext) -> Option<PromptHookMutation> {
        None
    }
}

pub trait SystemPromptComposeHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &PromptHookContext) -> Option<PromptHookMutation> {
        None
    }
}

pub trait ToolPromptComposeHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &PromptHookContext) -> Option<PromptHookMutation> {
        None
    }
}

pub trait PromptFinalizeHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &PromptHookContext) -> Option<PromptHookMutation> {
        None
    }
}

pub trait PromptEstimateFinalizeHook: Send + Sync {
    fn id(&self) -> &str;
    fn on_event(&self, _context: &PromptHookContext) -> Option<PromptHookMutation> {
        None
    }
}

type PromptInputHookList = Mutex<Vec<Arc<dyn PromptInputHook>>>;
type PromptHistoryHookList = Mutex<Vec<Arc<dyn PromptHistoryHook>>>;
type PromptEstimateHistoryHookList = Mutex<Vec<Arc<dyn PromptEstimateHistoryHook>>>;
type SystemPromptComposeHookList = Mutex<Vec<Arc<dyn SystemPromptComposeHook>>>;
type ToolPromptComposeHookList = Mutex<Vec<Arc<dyn ToolPromptComposeHook>>>;
type PromptFinalizeHookList = Mutex<Vec<Arc<dyn PromptFinalizeHook>>>;
type PromptEstimateFinalizeHookList = Mutex<Vec<Arc<dyn PromptEstimateFinalizeHook>>>;

static PROMPT_INPUT_HOOKS: OnceLock<PromptInputHookList> = OnceLock::new();
static PROMPT_HISTORY_HOOKS: OnceLock<PromptHistoryHookList> = OnceLock::new();
static PROMPT_ESTIMATE_HISTORY_HOOKS: OnceLock<PromptEstimateHistoryHookList> = OnceLock::new();
static SYSTEM_PROMPT_COMPOSE_HOOKS: OnceLock<SystemPromptComposeHookList> = OnceLock::new();
static TOOL_PROMPT_COMPOSE_HOOKS: OnceLock<ToolPromptComposeHookList> = OnceLock::new();
static PROMPT_FINALIZE_HOOKS: OnceLock<PromptFinalizeHookList> = OnceLock::new();
static PROMPT_ESTIMATE_FINALIZE_HOOKS: OnceLock<PromptEstimateFinalizeHookList> = OnceLock::new();

pub struct PromptHookRegistry;

impl PromptHookRegistry {
    #[allow(non_snake_case)]
    pub fn registerPromptInputHook(hook: Arc<dyn PromptInputHook>) {
        Self::unregisterPromptInputHook(hook.id());
        PROMPT_INPUT_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterPromptInputHook(hook_id: &str) {
        remove_by_id(PROMPT_INPUT_HOOKS.get_or_init(|| Mutex::new(Vec::new())), hook_id, |hook| hook.id());
    }

    #[allow(non_snake_case)]
    pub fn registerPromptHistoryHook(hook: Arc<dyn PromptHistoryHook>) {
        Self::unregisterPromptHistoryHook(hook.id());
        PROMPT_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterPromptHistoryHook(hook_id: &str) {
        remove_by_id(PROMPT_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())), hook_id, |hook| hook.id());
    }

    #[allow(non_snake_case)]
    pub fn registerPromptEstimateHistoryHook(hook: Arc<dyn PromptEstimateHistoryHook>) {
        Self::unregisterPromptEstimateHistoryHook(hook.id());
        PROMPT_ESTIMATE_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterPromptEstimateHistoryHook(hook_id: &str) {
        remove_by_id(PROMPT_ESTIMATE_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())), hook_id, |hook| hook.id());
    }

    #[allow(non_snake_case)]
    pub fn registerSystemPromptComposeHook(hook: Arc<dyn SystemPromptComposeHook>) {
        Self::unregisterSystemPromptComposeHook(hook.id());
        SYSTEM_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterSystemPromptComposeHook(hook_id: &str) {
        remove_by_id(SYSTEM_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), hook_id, |hook| hook.id());
    }

    #[allow(non_snake_case)]
    pub fn registerToolPromptComposeHook(hook: Arc<dyn ToolPromptComposeHook>) {
        Self::unregisterToolPromptComposeHook(hook.id());
        TOOL_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterToolPromptComposeHook(hook_id: &str) {
        remove_by_id(TOOL_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), hook_id, |hook| hook.id());
    }

    #[allow(non_snake_case)]
    pub fn registerPromptFinalizeHook(hook: Arc<dyn PromptFinalizeHook>) {
        Self::unregisterPromptFinalizeHook(hook.id());
        PROMPT_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterPromptFinalizeHook(hook_id: &str) {
        remove_by_id(PROMPT_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), hook_id, |hook| hook.id());
    }

    #[allow(non_snake_case)]
    pub fn registerPromptEstimateFinalizeHook(hook: Arc<dyn PromptEstimateFinalizeHook>) {
        Self::unregisterPromptEstimateFinalizeHook(hook.id());
        PROMPT_ESTIMATE_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(hook);
    }

    #[allow(non_snake_case)]
    pub fn unregisterPromptEstimateFinalizeHook(hook_id: &str) {
        remove_by_id(PROMPT_ESTIMATE_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), hook_id, |hook| hook.id());
    }

    #[allow(non_snake_case)]
    pub fn dispatchPromptInputHooks(initial_context: PromptHookContext) -> PromptHookContext {
        dispatch(initial_context, PROMPT_INPUT_HOOKS.get_or_init(|| Mutex::new(Vec::new())), |hook, context| hook.on_event(context))
    }

    #[allow(non_snake_case)]
    pub fn dispatchPromptHistoryHooks(initial_context: PromptHookContext) -> PromptHookContext {
        dispatch(initial_context, PROMPT_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())), |hook, context| hook.on_event(context))
    }

    #[allow(non_snake_case)]
    pub fn dispatchPromptEstimateHistoryHooks(initial_context: PromptHookContext) -> PromptHookContext {
        dispatch(initial_context, PROMPT_ESTIMATE_HISTORY_HOOKS.get_or_init(|| Mutex::new(Vec::new())), |hook, context| hook.on_event(context))
    }

    #[allow(non_snake_case)]
    pub fn dispatchSystemPromptComposeHooks(initial_context: PromptHookContext) -> PromptHookContext {
        dispatch(initial_context, SYSTEM_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), |hook, context| hook.on_event(context))
    }

    #[allow(non_snake_case)]
    pub fn dispatchToolPromptComposeHooks(initial_context: PromptHookContext) -> PromptHookContext {
        dispatch(initial_context, TOOL_PROMPT_COMPOSE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), |hook, context| hook.on_event(context))
    }

    #[allow(non_snake_case)]
    pub fn dispatchPromptFinalizeHooks(initial_context: PromptHookContext) -> PromptHookContext {
        dispatch(initial_context, PROMPT_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), |hook, context| hook.on_event(context))
    }

    #[allow(non_snake_case)]
    pub fn dispatchPromptEstimateFinalizeHooks(initial_context: PromptHookContext) -> PromptHookContext {
        dispatch(initial_context, PROMPT_ESTIMATE_FINALIZE_HOOKS.get_or_init(|| Mutex::new(Vec::new())), |hook, context| hook.on_event(context))
    }
}

fn remove_by_id<THook, F>(hooks: &Mutex<Vec<Arc<THook>>>, hook_id: &str, id_of: F)
where
    THook: ?Sized,
    F: Fn(&Arc<THook>) -> &str,
{
    hooks.lock().unwrap().retain(|hook| id_of(hook) != hook_id);
}

fn dispatch<THook, F>(
    initial_context: PromptHookContext,
    hooks: &Mutex<Vec<Arc<THook>>>,
    invoke: F,
) -> PromptHookContext
where
    THook: ?Sized,
    F: Fn(&Arc<THook>, &PromptHookContext) -> Option<PromptHookMutation>,
{
    let snapshot = hooks.lock().unwrap().clone();
    let mut current = initial_context;
    for hook in snapshot {
        if let Some(mutation) = invoke(&hook, &current) {
            current = apply_mutation(current, mutation);
        }
    }
    current
}

fn apply_mutation(mut current: PromptHookContext, mutation: PromptHookMutation) -> PromptHookContext {
    if let Some(raw_input) = mutation.raw_input {
        current.raw_input = Some(raw_input);
    }
    if let Some(processed_input) = mutation.processed_input {
        current.processed_input = Some(processed_input);
    }
    if let Some(chat_history) = mutation.chat_history {
        current.chat_history = chat_history;
    }
    if let Some(prepared_history) = mutation.prepared_history {
        current.prepared_history = prepared_history;
    }
    if let Some(system_prompt) = mutation.system_prompt {
        current.system_prompt = Some(system_prompt);
    }
    if let Some(tool_prompt) = mutation.tool_prompt {
        current.tool_prompt = Some(tool_prompt);
    }
    if let Some(available_tools) = mutation.available_tools {
        current.available_tools = available_tools;
    }
    if !mutation.metadata.is_empty() {
        current.metadata.extend(mutation.metadata);
    }
    current
}
