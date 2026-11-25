//! Runtime-owned package executions with bounded foreground waits and resumable output.
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use operit_host_api::HostRuntimeTaskSchedulerHost;
use serde_json::{json, Value};

use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{ToolInvocationContext, ToolRuntimeContext};
use crate::tools::ToolResultDataClasses::{stringResultData, JsonResultData, ToolResultData};

pub const DEFAULT_WAIT_MS: u64 = 20_000;
const MAX_OUTPUT_JSON_BYTES: usize = 20_000;

tokio::task_local! {
    static CURRENT_EXECUTION: Arc<PackageExecution>;
}

/// Identifies the authenticated conversation that owns an execution and its output.
#[derive(Clone, PartialEq, Eq)]
struct ExecutionOwner {
    extensionOwner: String,
    chatId: Option<String>,
}

impl ExecutionOwner {
    /// Captures ownership independently of mutable package arguments.
    fn fromTurn(turn: &ToolRuntimeContext) -> Self {
        Self { extensionOwner: turn.extensionOwner.clone(), chatId: turn.callerChatId.clone() }
    }
}

/// Retains every accepted invocation until its runtime is released.
#[derive(Default)]
pub struct PackageExecutionManager {
    executions: Mutex<BTreeMap<String, Arc<PackageExecution>>>,
    changed: Arc<tokio::sync::Notify>,
}

/// Owns one package task's output and final result independently of a waiting model call.
pub struct PackageExecution {
    id: String,
    toolName: String,
    owner: ExecutionOwner,
    changed: Arc<tokio::sync::Notify>,
    state: Mutex<ExecutionState>,
}

#[derive(Default)]
struct ExecutionState {
    output: Vec<String>,
    delivered: usize,
    outputOffset: usize,
    finalDelivered: bool,
    result: Option<Result<Vec<ToolResult>, String>>,
}

impl PackageExecutionManager {
    /// Registers one task before scheduling it so even immediately completed output is observable.
    pub fn create(&self, toolName: String, context: &ToolInvocationContext) -> Result<Arc<PackageExecution>, String> {
        let turn = context.turnSnapshot().ok_or("Package execution requires an authenticated AI turn")?;
        let execution = Arc::new(PackageExecution {
            id: format!("package_{}", uuid::Uuid::new_v4().simple()),
            toolName,
            owner: ExecutionOwner::fromTurn(turn),
            changed: self.changed.clone(),
            state: Mutex::new(ExecutionState::default()),
        });
        self.executions.lock().map_err(|_| "Package executions are poisoned")?
            .insert(execution.id.clone(), execution.clone());
        Ok(execution)
    }

    /// Captures existing tasks only, excluding completed results already delivered to the model.
    pub fn select(&self, context: &ToolInvocationContext, id: Option<&str>) -> Result<Vec<Arc<PackageExecution>>, String> {
        let owner = ExecutionOwner::fromTurn(context.turnSnapshot().ok_or("package_wait requires an authenticated AI turn")?);
        self.selectOwned(&owner, id)
    }

    /// Applies native ownership to explicit lookups and conversation-wide snapshots alike.
    fn selectOwned(&self, owner: &ExecutionOwner, id: Option<&str>) -> Result<Vec<Arc<PackageExecution>>, String> {
        let executions = self.executions.lock().map_err(|_| "Package executions are poisoned")?;
        if let Some(id) = id {
            let execution = executions.get(id).filter(|execution| &execution.owner == owner)
                .ok_or("Unknown package execution in this conversation")?;
            return Ok(vec![execution.clone()]);
        }
        Ok(executions.values().filter(|execution| &execution.owner == owner)
            .filter(|execution| !execution.state.lock().expect("Package execution is poisoned").finalDelivered)
            .cloned().collect())
    }

    /// Waits once for the selected snapshot, using a Host timer independent of package JS workers.
    pub async fn wait(
        &self, executions: &[Arc<PackageExecution>], scheduler: &dyn HostRuntimeTaskSchedulerHost,
        waitMs: u64, includeOutput: bool,
    ) -> Result<ToolResult, String> {
        let deadline = scheduler.monotonicTimeMillis().map_err(|error| error.to_string())?
            .checked_add(waitMs).ok_or("Package wait deadline overflow")?;
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let ready = executions.iter().find(|execution| execution.ready(includeOutput));
            let now = scheduler.monotonicTimeMillis().map_err(|error| error.to_string())?;
            if executions.is_empty() || ready.is_some() || now >= deadline {
                let selected = match ready { Some(execution) => Some(execution), None => executions.first() };
                let mut result = match selected {
                    Some(execution) => execution.receipt(includeOutput),
                    None => ToolResult { toolName: "package_wait".into(), success: true,
                        result: ToolResultData::JsonResultData(JsonResultData { value: json!({"status": "idle"}) }), error: None },
                };
                let pending = executions.iter().filter(|execution|
                    !execution.state.lock().expect("Package execution is poisoned").finalDelivered).count();
                let currentPending = selected.is_some_and(|execution|
                    !execution.state.lock().expect("Package execution is poisoned").finalDelivered);
                if pending > usize::from(currentPending) {
                    if let ToolResultData::JsonResultData(data) = &mut result.result {
                        data.value["pending_execution_count"] = json!(pending);
                    }
                }
                return Ok(result);
            }
            tokio::select! {
                _ = &mut changed => {},
                result = scheduler.waitForHostRuntimeDelay(deadline - now) => { result.map_err(|error| error.to_string())?; },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the public JSON payload rather than the retained execution state.
    fn payload(result: ToolResult) -> Value {
        serde_json::from_str(&result.result.toString()).unwrap()
    }

    /// Creates one authenticated scope without consulting runtime storage or package metadata.
    fn owner(chat: &str) -> ExecutionOwner {
        ExecutionOwner { extensionOwner: "config".into(), chatId: Some(chat.into()) }
    }

    /// Registers a controlled execution while using the production receipt and wait implementation.
    fn execution(manager: &PackageExecutionManager, id: &str, chat: &str) -> Arc<PackageExecution> {
        let execution = Arc::new(PackageExecution {
            id: id.into(), toolName: "fixture:slow".into(), owner: owner(chat),
            changed: manager.changed.clone(), state: Mutex::new(ExecutionState::default()),
        });
        manager.executions.lock().unwrap().insert(id.into(), execution.clone());
        execution
    }

    /// Returns an ordinary package completion, preserving the existing result and error contract.
    fn result() -> Result<Vec<ToolResult>, String> {
        Ok(vec![ToolResult { toolName: "fixture:slow".into(), success: true, result: stringResultData("done"), error: None }])
    }

    /// Verifies a bounded wait does not complete, cancel or lose the original execution.
    #[tokio::test]
    async fn wait_expiry_retains_execution_and_collects_late_output_and_completion() {
        let manager = PackageExecutionManager::default();
        let execution = execution(&manager, "one", "chat");
        let scheduler = operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new();
        let first = payload(manager.wait(&[execution.clone()], &scheduler, 5, false).await.unwrap());
        assert_eq!(first, json!({"execution_id":"one", "tool_name":"fixture:slow", "status":"running"}));
        execution.clone().scope(async {
            PackageExecution::outputCallback().unwrap()("first".into());
        }).await;
        let next = payload(manager.wait(&[execution.clone()], &scheduler, 20_000, true).await.unwrap());
        assert_eq!(next["output"][0]["value"], "first");
        let empty = payload(manager.wait(&[execution.clone()], &scheduler, 0, true).await.unwrap());
        assert_eq!(empty, first);
        execution.finish(result());
        let completed = payload(manager.wait(&[execution], &scheduler, 20_000, true).await.unwrap());
        assert_eq!(completed, json!({"execution_id":"one", "tool_name":"fixture:slow", "status":"completed", "result":"done"}));
        assert!(manager.selectOwned(&owner("chat"), None).unwrap().is_empty());
        assert_eq!(manager.selectOwned(&owner("chat"), Some("one")).unwrap().len(), 1);
    }

    /// Prevents foreign chats and configuration owners from reading or listing another execution.
    #[test]
    fn execution_lookup_enforces_chat_and_configuration_ownership() {
        let manager = PackageExecutionManager::default();
        execution(&manager, "one", "chat-a");
        assert!(manager.selectOwned(&owner("chat-b"), None).unwrap().is_empty());
        assert!(manager.selectOwned(&owner("chat-b"), Some("one")).is_err());
        let mut foreign = owner("chat-a");
        foreign.extensionOwner = "another-config".into();
        assert!(manager.selectOwned(&foreign, Some("one")).is_err());
    }

    /// Collects completed-but-unread results with running tasks under one shared wait budget.
    #[tokio::test]
    async fn wait_snapshot_collects_unread_completions_without_waiting_for_other_tasks() {
        let manager = PackageExecutionManager::default();
        let done = execution(&manager, "done", "chat");
        let running = execution(&manager, "running", "chat");
        done.finish(result());
        let selected = manager.selectOwned(&owner("chat"), None).unwrap();
        execution(&manager, "later", "chat");
        assert_eq!(selected.len(), 2);
        let scheduler = operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new();
        let output = payload(manager.wait(&selected, &scheduler, 20_000, true).await.unwrap());
        assert_eq!(output["execution_id"], "done");
        assert_eq!(output["pending_execution_count"], 1);
        assert!(running.state.lock().unwrap().result.is_none());
        assert_eq!(manager.selectOwned(&owner("chat"), None).unwrap().len(), 2);
    }

    /// Preserves late failures and ensures twenty seconds is always the maximum wait.
    #[test]
    fn failed_execution_and_wait_budget_are_explicit() {
        let manager = PackageExecutionManager::default();
        let failed = execution(&manager, "failed", "chat");
        failed.finish(Err("actual error".into()));
        let result = failed.receipt(false);
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("actual error"));
        let observed = payload(failed.receipt(true));
        assert_eq!(observed["error"], "actual error");
        assert_eq!(observed["status"], "failed");
        assert_eq!(waitBudget(None).unwrap(), 20_000);
        assert_eq!(waitBudget(Some(&json!("0"))).unwrap(), 0);
        for value in [json!(-1), json!(20_001), json!(0.5), json!("invalid")] {
            assert!(waitBudget(Some(&value)).is_err());
        }
    }

    /// Reads an oversized Unicode output event in complete JSON pages without losing any fragment.
    #[test]
    fn large_intermediate_output_remains_readable_after_completion() {
        let manager = PackageExecutionManager::default();
        let execution = execution(&manager, "large", "chat");
        let original = "雪🌍\n\"\\".repeat(20_000);
        execution.state.lock().unwrap().output.push(original.clone());
        execution.finish(result());
        let mut received = String::new();
        loop {
            let page = payload(execution.receipt(true));
            assert!(serde_json::to_vec(&page["output"]).unwrap().len() <= MAX_OUTPUT_JSON_BYTES + 2);
            for output in page["output"].as_array().unwrap() {
                assert_eq!(output["sequence"], 1);
                assert_eq!(output["offset"].as_u64().unwrap() as usize, received.len());
                received.push_str(output["value"].as_str().unwrap());
            }
            if page["has_more_output"] != true { break; }
            assert_eq!(manager.selectOwned(&owner("chat"), None).unwrap().len(), 1);
        }
        assert_eq!(received, original);
        assert!(manager.selectOwned(&owner("chat"), None).unwrap().is_empty());
    }

    /// Returns completed business data directly without reinterpreting package-owned keys or strings.
    #[test]
    fn ordinary_completion_preserves_business_json_and_reports_errors_once() {
        use crate::ConversationMarkupManager::ConversationMarkupManager;
        let manager = PackageExecutionManager::default();
        let execution = execution(&manager, "json", "chat");
        let value = json!({"__type":"PackageOwnedType", "nested":[null,false], "text":"{\"x\":1}"});
        execution.finish(Ok(vec![ToolResult { toolName: "fixture:slow".into(), success: true,
            result: ToolResultData::JsonResultData(JsonResultData { value: value.clone() }), error: None }]));
        assert_eq!(payload(execution.receipt(false)), value);
        assert_eq!(payload(execution.receipt(true))["result"], value);
        execution.finish(Ok(vec![ToolResult { toolName: "fixture:slow".into(), success: false,
            result: stringResultData(""), error: Some("setting does not exist".into()) }]));
        let failed = execution.receipt(false);
        let message = ConversationMarkupManager::formatToolResultForMessage(&failed);
        assert!(!failed.success);
        assert_eq!(message.matches("setting does not exist").count(), 1);
        assert!(!message.contains("execution_id"));
    }

    /// Keeps native UI data for ordinary completions and structured, untagged values in wait receipts.
    #[test]
    fn native_result_retains_renderer_data_and_wait_does_not_encode_json_twice() {
        use crate::tools::ToolResultDataClasses::DirectoryListingData;
        let manager = PackageExecutionManager::default();
        let execution = execution(&manager, "native", "chat");
        execution.finish(Ok(vec![ToolResult { toolName: "fixture:slow".into(), success: true,
            result: ToolResultData::DirectoryListingData(DirectoryListingData { path: "/files".into(), entries: vec![] }), error: None }]));
        assert!(matches!(execution.receipt(false).result, ToolResultData::DirectoryListingData(_)));
        assert_eq!(payload(execution.receipt(true))["result"], json!({"path":"/files", "entries":[]}));
    }

    /// Keeps intermediate output on failures while reporting the actual error exactly once.
    #[test]
    fn failed_completion_with_output_preserves_foreground_and_wait_error_contracts() {
        use crate::ConversationMarkupManager::ConversationMarkupManager;
        let manager = PackageExecutionManager::default();
        let execution = execution(&manager, "failed-output", "chat");
        execution.state.lock().unwrap().output.push("progress before failure".into());
        execution.finish(Ok(vec![ToolResult { toolName: "fixture:slow".into(), success: false,
            result: stringResultData(""), error: Some("actual failure".into()) }]));
        let foreground = execution.receipt(false);
        assert!(!foreground.success);
        let message = ConversationMarkupManager::formatToolResultForMessage(&foreground);
        assert_eq!(message.matches("actual failure").count(), 1);
        assert!(message.contains("progress before failure"));
        let observed = execution.receipt(true);
        assert!(observed.success, "Observing a failed operation is a successful read");
        assert_eq!(payload(observed), json!({"execution_id":"failed-output", "tool_name":"fixture:slow",
            "status":"failed", "error":"actual failure"}));
    }

    /// Preserves identity, structured business values and individual failures in multi-result calls.
    #[test]
    fn multiple_results_keep_their_names_values_and_errors() {
        let manager = PackageExecutionManager::default();
        let execution = execution(&manager, "multiple", "chat");
        execution.finish(Ok(vec![
            ToolResult { toolName: "fixture:first".into(), success: true,
                result: ToolResultData::JsonResultData(JsonResultData { value: json!({"items":[null,false]}) }), error: None },
            ToolResult { toolName: "fixture:second".into(), success: false,
                result: stringResultData("details"), error: Some("second failed".into()) },
        ]));
        let observed = payload(execution.receipt(true));
        assert_eq!(observed["status"], "failed");
        assert_eq!(observed["results"], json!([
            {"tool_name":"fixture:first", "success":true, "result":{"items":[null,false]}},
            {"tool_name":"fixture:second", "success":false, "result":"details", "error":"second failed"},
        ]));
        assert!(observed.get("result").is_none());
    }

    /// Observes an empty selection immediately without emitting empty arrays or zero counters.
    #[tokio::test]
    async fn empty_wait_reports_idle() {
        let manager = PackageExecutionManager::default();
        let scheduler = operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new();
        let result = manager.wait(&[], &scheduler, 20_000, true).await.unwrap();
        assert_eq!(result.toolName, "package_wait");
        assert!(result.success);
        assert_eq!(payload(result), json!({"status":"idle"}));
    }
}

impl PackageExecution {
    /// Runs the entire invocation under the execution-owned output scope.
    pub async fn scope<T>(self: Arc<Self>, future: impl std::future::Future<Output = T>) -> T {
        CURRENT_EXECUTION.scope(self, future).await
    }

    /// Captures a callback while the package invocation is scoped, before entering its JS engine.
    pub fn outputCallback() -> Option<Arc<dyn Fn(String) + Send + Sync>> {
        CURRENT_EXECUTION.try_with(|execution| {
            let execution = execution.clone();
            Arc::new(move |value| {
                execution.state.lock().expect("Package execution is poisoned").output.push(value);
                execution.changed.notify_waiters();
            }) as Arc<dyn Fn(String) + Send + Sync>
        }).ok()
    }

    /// Records actual completion once; foreground wait expiry never enters this path.
    pub fn finish(&self, result: Result<Vec<ToolResult>, String>) {
        self.state.lock().expect("Package execution is poisoned").result = Some(result);
        self.changed.notify_waiters();
    }

    /// Reports completion or unread output without consuming either.
    fn ready(&self, includeOutput: bool) -> bool {
        let state = self.state.lock().expect("Package execution is poisoned");
        state.result.is_some() || (includeOutput && state.delivered < state.output.len())
    }

    /// Consumes only newly produced output while retaining the final result for explicit later reads.
    fn receipt(&self, forWait: bool) -> ToolResult {
        let mut state = self.state.lock().expect("Package execution is poisoned");
        let mut output = Vec::new();
        let mut budget = MAX_OUTPUT_JSON_BYTES;
        while state.delivered < state.output.len() && budget > 256 {
            let value = &state.output[state.delivered];
            let remaining = &value[state.outputOffset..];
            let length = outputFragmentLength(remaining, budget - 256);
            if length == 0 && !remaining.is_empty() { break; }
            let complete = length == remaining.len();
            let entry = json!({"sequence": state.delivered + 1, "offset": state.outputOffset,
                "value": &remaining[..length], "complete": complete});
            budget -= serde_json::to_vec(&entry).expect("Output fragment must serialize").len() + 1;
            output.push(entry);
            if complete {
                state.delivered += 1;
                state.outputOffset = 0;
            } else {
                state.outputOffset += length;
            }
        }
        let hasMore = state.delivered < state.output.len();
        state.finalDelivered = state.result.is_some() && !hasMore;
        // Ordinary completions retain their original result, renderer and failure status.
        if !forWait && output.is_empty() && !hasMore {
            match &state.result {
                Some(Ok(results)) if results.len() == 1 => return results[0].clone(),
                Some(Err(error)) => return ToolResult { toolName: self.toolName.clone(),
                    success: false, result: stringResultData(""), error: Some(error.clone()) },
                _ => {}
            }
        }
        let status = match &state.result {
            None => "running",
            Some(Ok(results)) if results.iter().all(|result| result.success) => "completed",
            _ => "failed",
        };
        let success = forWait || status != "failed";
        let error = if success { None } else {
            Some(match &state.result {
                Some(Ok(results)) => results.iter().filter_map(|result| result.error.as_deref()).collect::<Vec<_>>().join("\n"),
                Some(Err(error)) => error.clone(),
                None => unreachable!("Running execution cannot fail"),
            })
        };
        let mut value = json!({"execution_id": self.id, "tool_name": self.toolName, "status": status});
        if !output.is_empty() { value["output"] = json!(output); }
        if hasMore { value["has_more_output"] = json!(true); }
        match &state.result {
            Some(Ok(results)) if results.len() == 1 => {
                let result = &results[0];
                if result.success || !matches!(&result.result, ToolResultData::StringResultData(data) if data.value.is_empty()) {
                    value["result"] = modelResultValue(&result.result);
                }
                if forWait {
                    if let Some(error) = &result.error { value["error"] = json!(error); }
                }
            }
            Some(Ok(results)) => value["results"] = Value::Array(results.iter().map(|result| {
                let mut entry = json!({"tool_name": result.toolName, "success": result.success,
                    "result": modelResultValue(&result.result)});
                if forWait {
                    if let Some(error) = &result.error { entry["error"] = json!(error); }
                }
                entry
            }).collect()),
            Some(Err(error)) if forWait => value["error"] = json!(error),
            Some(Err(_)) => {},
            None => {}
        }
        ToolResult { toolName: if forWait { "package_wait".into() } else { self.toolName.clone() },
            success, result: ToolResultData::JsonResultData(JsonResultData { value }), error }
    }
}

/// Keeps business JSON structured and removes only the native result's own serialization tag.
fn modelResultValue(result: &ToolResultData) -> Value {
    match result {
        ToolResultData::JsonResultData(data) => data.value.clone(),
        ToolResultData::StringResultData(data) => json!(data.value),
        ToolResultData::BooleanResultData(data) => json!(data.value),
        ToolResultData::IntResultData(data) => json!(data.value),
        ToolResultData::BinaryResultData(_) => json!(result.toString()),
        _ => {
            let mut value = serde_json::to_value(result).expect("Tool result must serialize");
            value.as_object_mut().expect("Native result is an object").remove("__type");
            value
        }
    }
}

/// Fits a Unicode-safe fragment inside the actual JSON string budget, including escaped characters.
fn outputFragmentLength(value: &str, budget: usize) -> usize {
    let mut used = 2;
    let mut bytes = 0;
    for character in value.chars() {
        let cost = match character {
            '"' | '\\' | '\n' | '\r' | '\t' | '\u{08}' | '\u{0c}' => 2,
            '\u{00}'..='\u{1f}' => 6,
            _ => character.len_utf8(),
        };
        if used + cost > budget { break; }
        used += cost;
        bytes += character.len_utf8();
    }
    bytes
}

/// Decodes a foreground wait budget; no model request can extend it beyond twenty seconds.
pub fn waitBudget(value: Option<&Value>) -> Result<u64, String> {
    match value {
        None => Ok(DEFAULT_WAIT_MS),
        Some(value) => match value {
            Value::Number(value) => value.as_u64(),
            Value::String(value) => value.parse::<u64>().ok(),
            _ => None,
        }.filter(|value| *value <= DEFAULT_WAIT_MS)
            .ok_or_else(|| "timeout_ms must be an integer from 0 to 20000".into()),
    }
}

/// Implements package_wait without entering any package engine or invoking the package again.
pub struct PackageWaitExecutor;

impl crate::ToolExecutionManager::SharedToolExecutor for PackageWaitExecutor {
    /// Validates wait controls and snapshots the conversation's existing executions before waiting.
    fn prepare<'a>(
        &'a self, request: &'a crate::ToolExecutionManager::ToolCall,
        context: &'a ToolInvocationContext, checkAccess: bool,
    ) -> Result<crate::ToolExecutionManager::PreparedToolInvocation<'a>, String> {
        for key in request.parameters.keys() {
            if !matches!(key.as_str(), "execution_id" | "timeout_ms") { return Err(format!("Unexpected package_wait parameter: {key}")); }
        }
        let id = request.parameters.get("execution_id").map(|value| value.as_str()
            .filter(|value| !value.is_empty() && value.trim() == *value)
            .ok_or("execution_id must be exact nonblank text")).transpose()?;
        let waitMs = waitBudget(request.parameters.get("timeout_ms"))?;
        let manager = context.host().packageExecutions();
        let executions = manager.select(context, id)?;
        let scheduler = context.host().getContext().hostRuntimeTaskSchedulerHost
            .ok_or("Runtime task scheduler host is unavailable")?;
        Ok(crate::tools::ToolInvocation::prepareInvocation(request, checkAccess,
            crate::ToolExecutionManager::ToolEffect::READ, async move {
                Ok(manager.wait(&executions, scheduler.as_ref(), waitMs, true).await?.result)
            }))
    }
}
