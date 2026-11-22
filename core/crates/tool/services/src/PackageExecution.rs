//! Runtime-owned package executions with bounded foreground waits and resumable output.
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use operit_host_api::HostRuntimeTaskSchedulerHost;
use serde_json::{json, Value};

use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{ToolInvocationContext, ToolRuntimeContext};

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
    ) -> Result<Value, String> {
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
                let entries = selected.into_iter().map(|execution| execution.receipt()).collect::<Vec<_>>();
                let pending = executions.iter().filter(|execution|
                    !execution.state.lock().expect("Package execution is poisoned").finalDelivered).count();
                return Ok(json!({"executions": entries, "pending_execution_count": pending,
                    "timed_out": !executions.is_empty() && ready.is_none() && now >= deadline}));
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
    use crate::tools::ToolResultDataClasses::stringResultData;

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
        let first = manager.wait(&[execution.clone()], &scheduler, 5, false).await.unwrap();
        assert_eq!(first["timed_out"], true);
        assert_eq!(first["executions"][0]["status"], "running");
        execution.clone().scope(async {
            PackageExecution::outputCallback().unwrap()("first".into());
        }).await;
        let next = manager.wait(&[execution.clone()], &scheduler, 20_000, true).await.unwrap();
        assert_eq!(next["executions"][0]["output"][0]["value"], "first");
        assert_eq!(next["executions"][0]["cursor"], 1);
        let empty = manager.wait(&[execution.clone()], &scheduler, 0, true).await.unwrap();
        assert_eq!(empty["executions"][0]["output"], json!([]));
        execution.finish(result());
        let completed = manager.wait(&[execution], &scheduler, 20_000, true).await.unwrap();
        assert_eq!(completed["executions"][0]["status"], "completed");
        assert_eq!(completed["executions"][0]["results"][0]["result"], "done");
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
        let output = manager.wait(&selected, &scheduler, 20_000, true).await.unwrap();
        assert_eq!(output["timed_out"], false);
        assert_eq!(output["executions"].as_array().unwrap().len(), 1);
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
        assert_eq!(failed.receipt()["error"], "actual error");
        assert_eq!(failed.receipt()["status"], "failed");
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
            let page = execution.receipt();
            assert!(serde_json::to_vec(&page["output"]).unwrap().len() <= MAX_OUTPUT_JSON_BYTES + 2);
            for output in page["output"].as_array().unwrap() {
                assert_eq!(output["sequence"], 1);
                assert_eq!(output["offset"].as_u64().unwrap() as usize, received.len());
                received.push_str(output["value"].as_str().unwrap());
            }
            if page["has_more_output"] == false { break; }
            assert_eq!(manager.selectOwned(&owner("chat"), None).unwrap().len(), 1);
        }
        assert_eq!(received, original);
        assert!(manager.selectOwned(&owner("chat"), None).unwrap().is_empty());
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
    fn receipt(&self) -> Value {
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
        let (status, results, error) = match &state.result {
            None => ("running", Vec::new(), None),
            Some(Ok(results)) => (
                if results.iter().all(|result| result.success) { "completed" } else { "failed" },
                results.iter().map(|result| json!({
                    "tool_name": result.toolName, "success": result.success,
                    "result": result.result.toString(), "error": result.error,
                })).collect(),
                None,
            ),
            Some(Err(error)) => ("failed", Vec::new(), Some(error.clone())),
        };
        let hasMore = state.delivered < state.output.len();
        state.finalDelivered = state.result.is_some() && !hasMore;
        json!({"execution_id": self.id, "tool_name": self.toolName, "status": status,
            "output": output, "cursor": state.delivered, "has_more_output": hasMore,
            "results": results, "error": error})
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
                let value = manager.wait(&executions, scheduler.as_ref(), waitMs, true).await?;
                Ok(crate::tools::ToolResultDataClasses::ToolResultData::JsonResultData(
                    crate::tools::ToolResultDataClasses::JsonResultData { value }))
            }))
    }
}
