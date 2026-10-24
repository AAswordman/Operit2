//! Process-owned work information shared by every chat runtime and presentation host.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use operit_host_api::HostManager::HostManager;
use operit_model::InputProcessingState::InputProcessingState;
use operit_store::PreferencesDataStore::StateFlow;
use serde::{Deserialize, Serialize};

use super::core::MessageProcessingDelegate::MessageProcessingDelegate;
use super::RuntimeHostInteractionService::RuntimeHostInteractionToolPermissionRequest;

/// Describes a step's measurable progress without implying whole-work completion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkStepProgress {
    pub value: f32,
    pub scope: String,
}

/// Identifies one real execution independently of its conversation and visible window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkActivity {
    pub workId: String,
    pub chatId: String,
    pub activation: BTreeMap<String, String>,
    pub runId: u64,
    pub title: String,
    pub status: String,
    pub phase: String,
    pub detail: String,
    pub startedAtMillis: i64,
    pub endedAtMillis: Option<i64>,
    pub progress: Option<WorkStepProgress>,
    pub permissionRequestIds: Vec<String>,
    pub canStop: bool,
}

/// Publishes a complete ordered revision so hosts can reject stale deliveries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkActivitySnapshot {
    pub schemaVersion: u32,
    pub revision: u64,
    pub works: Vec<WorkActivity>,
}

/// Retains the exact execution owner used by an explicit stop action.
struct WorkController {
    delegate: MessageProcessingDelegate,
    chatId: String,
    turnId: u64,
}

/// Serializes work lifecycle changes and pending human decisions.
#[derive(Default)]
struct WorkState {
    revision: u64,
    works: BTreeMap<String, WorkActivity>,
    controllers: BTreeMap<String, WorkController>,
    permissions: Vec<RuntimeHostInteractionToolPermissionRequest>,
    permissionOwners: BTreeMap<String, String>,
}

/// Keeps observation alive independently of Flutter engines and activities.
struct WorkHub {
    publication: Mutex<()>,
    state: Mutex<WorkState>,
    updates: StateFlow<String>,
}

/// Returns the sole process-owned execution presentation registry.
fn hub() -> &'static WorkHub {
    static HUB: OnceLock<WorkHub> = OnceLock::new();
    HUB.get_or_init(|| WorkHub {
        publication: Mutex::new(()),
        state: Mutex::new(WorkState::default()),
        updates: StateFlow::new(
            serde_json::to_string(&WorkActivitySnapshot {
                schemaVersion: 1,
                revision: 0,
                works: Vec::new(),
            })
            .expect("initial work snapshot must serialize"),
        ),
    })
}

/// Orders attention requests before running work, then retains actual terminal results.
fn snapshot(state: &WorkState) -> WorkActivitySnapshot {
    let mut works = state.works.values().cloned().collect::<Vec<_>>();
    for work in &mut works {
        if work.endedAtMillis.is_none() {
            let pending = state
                .permissions
                .iter()
                .filter(|request| {
                    state.permissionOwners.get(&request.requestId) == Some(&work.workId)
                })
                .collect::<Vec<_>>();
            work.permissionRequestIds = pending
                .iter()
                .map(|request| request.requestId.clone())
                .collect();
            if let Some(request) = pending.first() {
                work.status = "waiting".to_string();
                work.detail = request.description.clone();
                work.progress = None;
            }
        }
    }
    works.sort_by(|left, right| {
        let rank = |work: &WorkActivity| match work.status.as_str() {
            "waiting" => 0,
            "running" => 1,
            _ => 2,
        };
        rank(left)
            .cmp(&rank(right))
            .then_with(|| match (left.endedAtMillis, right.endedAtMillis) {
                (Some(left), Some(right)) => right.cmp(&left),
                _ => left.startedAtMillis.cmp(&right.startedAtMillis),
            })
            .then_with(|| left.workId.cmp(&right.workId))
    });
    WorkActivitySnapshot {
        schemaVersion: 1,
        revision: state.revision,
        works,
    }
}

/// Commits a change before notifying subscribers, preserving publication order.
fn change(action: impl FnOnce(&mut WorkState)) {
    let owner = hub();
    let _publication = owner
        .publication
        .lock()
        .expect("work publication mutex poisoned");
    let encoded = {
        let mut state = owner.state.lock().expect("work state mutex poisoned");
        let previous = snapshot(&state);
        action(&mut state);
        if snapshot(&state) == previous {
            return;
        }
        state.revision = state
            .revision
            .checked_add(1)
            .expect("work revision overflow");
        // Completed records are bounded; active work and its controllers are never evicted.
        let mut ended = state
            .works
            .values()
            .filter_map(|work| work.endedAtMillis.map(|end| (end, work.workId.clone())))
            .collect::<Vec<_>>();
        ended.sort();
        let excess = ended.len().saturating_sub(32);
        for (_, id) in ended.into_iter().take(excess) {
            state.works.remove(&id);
        }
        serde_json::to_string(&snapshot(&state)).expect("work snapshot must serialize")
    };
    owner.updates.set_value(encoded);
}

/// Exposes platform-neutral work information through the ordinary generated Core API.
pub struct WorkActivityService;

impl WorkActivityService {
    /// Constructs a process facade without binding work to an individual UI subscriber.
    pub fn getInstance(_context: &HostManager) -> Self {
        Self
    }

    /// Observes complete revisions covering main, floating, and detached executions.
    pub fn workUpdatesFlow(&self) -> StateFlow<String> {
        hub().updates.clone()
    }

    /// Returns the latest committed work revision for explicit host synchronization.
    pub fn workSnapshot(&self) -> String {
        hub().updates.value()
    }

    /// Stops the exact running generation selected by the user.
    pub async fn stopWork(&self, workId: String) -> Result<(), String> {
        let (mut delegate, chatId, turnId) = {
            let state = hub().state.lock().map_err(|error| error.to_string())?;
            let work = state
                .works
                .get(&workId)
                .ok_or_else(|| format!("Unknown work: {workId}"))?;
            if !work.canStop {
                return Err(format!("Work cannot be stopped: {workId}"));
            }
            let controller = state
                .controllers
                .get(&workId)
                .ok_or_else(|| format!("Work is no longer stoppable: {workId}"))?;
            (
                controller.delegate.clone_for_core(),
                controller.chatId.clone(),
                controller.turnId,
            )
        };
        delegate.cancelWorkGeneration(chatId, turnId).await
    }
}

/// Starts one work item with the title and timestamps captured from its real execution.
pub(crate) fn beginWork(
    workId: String,
    chatId: String,
    title: String,
    turnId: u64,
    startedAtMillis: i64,
    delegate: MessageProcessingDelegate,
) {
    change(|state| {
        state.controllers.insert(
            workId.clone(),
            WorkController {
                delegate,
                chatId: chatId.clone(),
                turnId,
            },
        );
        state.works.insert(
            workId.clone(),
            WorkActivity {
                activation: BTreeMap::from([
                    ("type".to_string(), "open_chat".to_string()),
                    ("chatId".to_string(), chatId.clone()),
                ]),
                workId,
                chatId,
                runId: turnId,
                title,
                status: "running".to_string(),
                phase: "processing".to_string(),
                detail: String::new(),
                startedAtMillis,
                endedAtMillis: None,
                progress: None,
                permissionRequestIds: Vec::new(),
                canStop: true,
            },
        );
    });
}

/// Updates a registered execution without creating work from incidental UI state.
pub(crate) fn updateWork(workId: &str, input: &InputProcessingState) {
    change(|state| {
        let Some(work) = state.works.get_mut(workId) else {
            return;
        };
        if work.endedAtMillis.is_some() {
            return;
        }
        work.detail.clear();
        work.progress = None;
        work.phase = match input {
            InputProcessingState::Processing { .. } => "processing",
            InputProcessingState::Connecting { .. } => "connecting",
            InputProcessingState::Receiving { .. } => "receiving",
            InputProcessingState::ExecutingTool { toolName }
            | InputProcessingState::ProcessingToolResult { toolName } => {
                work.detail = toolName.clone();
                if matches!(input, InputProcessingState::ExecutingTool { .. }) {
                    "executingTool"
                } else {
                    "processingToolResult"
                }
            }
            InputProcessingState::ToolProgress {
                toolName,
                progress,
                message,
            } => {
                work.detail = message.clone();
                // InputProcessingState measures a step from zero to one; the presentation wire uses percent.
                work.progress = Some(WorkStepProgress {
                    value: *progress * 100.0,
                    scope: toolName.clone(),
                });
                "executingTool"
            }
            InputProcessingState::Summarizing { .. } => "summarizing",
            InputProcessingState::ExecutingPlan { .. } => "executingPlan",
            InputProcessingState::Completed
            | InputProcessingState::Error { .. }
            | InputProcessingState::Idle => {
                work.status = match input {
                    InputProcessingState::Completed => "completed",
                    InputProcessingState::Error { .. } => "failed",
                    InputProcessingState::Idle => "cancelled",
                    _ => unreachable!("terminal work state already matched"),
                }
                .to_string();
                work.endedAtMillis = Some(operit_host_api::TimeUtils::currentTimeMillis());
                work.canStop = false;
                state.controllers.remove(workId);
                "ended"
            }
        }
        .to_string();
    });
}

/// Binds each attention request to its original execution before later runs can reuse the chat.
fn assignPermissions(
    state: &mut WorkState,
    requests: Vec<RuntimeHostInteractionToolPermissionRequest>,
) {
    let requestIds = requests
        .iter()
        .map(|request| request.requestId.clone())
        .collect::<std::collections::BTreeSet<_>>();
    state
        .permissionOwners
        .retain(|requestId, _| requestIds.contains(requestId));
    for request in &requests {
        if state.permissionOwners.contains_key(&request.requestId) {
            continue;
        }
        if let Some(work) = state
            .works
            .values()
            .find(|work| work.chatId == request.chatId && work.endedAtMillis.is_none())
        {
            state
                .permissionOwners
                .insert(request.requestId.clone(), work.workId.clone());
        }
    }
    state.permissions = requests;
}

/// Updates human-attention information from the actual permission broker.
pub(crate) fn permissionsChanged(requests: Vec<RuntimeHostInteractionToolPermissionRequest>) {
    change(|state| assignPermissions(state, requests));
}

/// Registers a native owner without requiring a Flutter event receiver.
pub fn subscribeWorkUpdates(subscriber: impl FnMut(String) + Send + 'static) -> usize {
    hub().updates.subscribe(subscriber)
}

/// Releases a native subscription when its owning Core process bridge shuts down.
pub fn unsubscribeWorkUpdates(subscriptionId: usize) {
    hub().updates.unsubscribe(subscriptionId);
}

/// Owns failure publication until a send request hands work to its completion stream.
pub(crate) struct WorkRequestGuard {
    workId: Option<String>,
}

impl WorkRequestGuard {
    /// Arms request cleanup for a newly published execution.
    pub(crate) fn new(workId: String) -> Self {
        Self {
            workId: Some(workId),
        }
    }

    /// Transfers lifecycle ownership to the already-established response completion path.
    pub(crate) fn handoff(&mut self) {
        self.workId = None;
    }
}

impl Drop for WorkRequestGuard {
    /// Ends requests whose future failed or was dropped before completion ownership was installed.
    fn drop(&mut self) {
        if let Some(workId) = self.workId.take() {
            updateWork(
                &workId,
                &InputProcessingState::Error {
                    message: "Work request ended before response ownership was established"
                        .to_string(),
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::RuntimeHostInteractionService::RuntimeHostInteractionToolPermissionTool;
    use super::*;

    /// Builds a real wire record for snapshot ordering and lifecycle checks.
    fn work(id: &str, chat: &str, started: i64) -> WorkActivity {
        WorkActivity {
            workId: id.to_string(),
            chatId: chat.to_string(),
            activation: BTreeMap::from([
                ("type".to_string(), "open_chat".to_string()),
                ("chatId".to_string(), chat.to_string()),
            ]),
            runId: 1,
            title: "Organize documents".to_string(),
            status: "running".to_string(),
            phase: "executingTool".to_string(),
            detail: "Search".to_string(),
            startedAtMillis: started,
            endedAtMillis: None,
            progress: Some(WorkStepProgress {
                value: 40.0,
                scope: "search".to_string(),
            }),
            permissionRequestIds: Vec::new(),
            canStop: true,
        }
    }

    /// Builds a permission request without exposing its tool arguments in the work snapshot.
    fn permission(chat: &str) -> RuntimeHostInteractionToolPermissionRequest {
        RuntimeHostInteractionToolPermissionRequest {
            requestId: "permission-1".to_string(),
            chatId: chat.to_string(),
            tool: RuntimeHostInteractionToolPermissionTool {
                name: "move_files".to_string(),
                parameters: Vec::new(),
            },
            description: "Allow moving 12 files?".to_string(),
            requestedAtMillis: 50,
        }
    }

    /// Verifies attention ordering while keeping original step state ready for resolution.
    #[test]
    fn attentionOverlaysOnlyItsOwnConversation() {
        let mut state = WorkState::default();
        state.works.insert("a".to_string(), work("a", "chat-a", 10));
        state.works.insert("b".to_string(), work("b", "chat-b", 20));
        assignPermissions(&mut state, vec![permission("chat-b")]);
        let visible = snapshot(&state);
        assert_eq!(visible.works[0].workId, "b");
        assert_eq!(visible.works[0].status, "waiting");
        assert_eq!(visible.works[0].detail, "Allow moving 12 files?");
        assert!(visible.works[0].progress.is_none());
        assert_eq!(visible.works[1].status, "running");
        state.permissions.clear();
        let resolved = snapshot(&state);
        assert_eq!(resolved.works[1].status, "running");
        assert_eq!(resolved.works[1].progress.as_ref().unwrap().value, 40.0);
        assert_eq!(resolved.works[1].startedAtMillis, 20);
    }

    /// Verifies a stopped run's pending decision cannot label a later run as waiting.
    #[test]
    fn oldPermissionDoesNotAttachToANewerRun() {
        let mut state = WorkState::default();
        state
            .works
            .insert("old".to_string(), work("old", "reused-chat", 10));
        assignPermissions(&mut state, vec![permission("reused-chat")]);
        let old = state.works.get_mut("old").unwrap();
        old.status = "cancelled".to_string();
        old.endedAtMillis = Some(20);
        old.canStop = false;
        state
            .works
            .insert("new".to_string(), work("new", "reused-chat", 30));
        assignPermissions(&mut state, vec![permission("reused-chat")]);
        let visible = snapshot(&state);
        assert_eq!(visible.works[0].workId, "new");
        assert_eq!(visible.works[0].status, "running");
        assert!(visible.works[0].permissionRequestIds.is_empty());
    }

    /// Verifies completed work never inherits a stale pending permission overlay.
    #[test]
    fn terminalResultsRemainTerminalWithPendingPermission() {
        let mut state = WorkState::default();
        let mut ended = work("ended", "chat-a", 10);
        ended.status = "completed".to_string();
        ended.endedAtMillis = Some(30);
        ended.canStop = false;
        state.works.insert(ended.workId.clone(), ended);
        state
            .works
            .insert("running".to_string(), work("running", "chat-b", 40));
        assignPermissions(&mut state, vec![permission("chat-a")]);
        let visible = snapshot(&state);
        assert_eq!(visible.works[0].workId, "running");
        assert_eq!(visible.works[1].status, "completed");
        assert_eq!(visible.works[1].endedAtMillis, Some(30));
        assert!(!visible.works[1].canStop);
    }

    /// Verifies step fractions become percentages and late callbacks cannot resurrect completion.
    #[test]
    fn lateProgressCannotResurrectCompletedWork() {
        let id = uuid::Uuid::new_v4().to_string();
        change(|state| {
            state
                .works
                .insert(id.clone(), work(&id, "test-progress-chat", 1));
        });
        updateWork(
            &id,
            &InputProcessingState::ToolProgress {
                toolName: "download".to_string(),
                progress: 0.6,
                message: "Receiving a file".to_string(),
            },
        );
        {
            let state = hub().state.lock().unwrap();
            assert!((state.works[&id].progress.as_ref().unwrap().value - 60.0).abs() < 0.001);
        }
        updateWork(&id, &InputProcessingState::Completed);
        let endedAt = hub().state.lock().unwrap().works[&id].endedAtMillis;
        updateWork(
            &id,
            &InputProcessingState::Receiving {
                message: "late callback".to_string(),
            },
        );
        let state = hub().state.lock().unwrap();
        assert_eq!(state.works[&id].status, "completed");
        assert_eq!(state.works[&id].endedAtMillis, endedAt);
        assert!(!state.works[&id].canStop);
    }

    /// Verifies stale system actions cannot reserve cancellation of a newer turn.
    #[test]
    fn staleWorkStopCannotCancelNewGeneration() {
        let mut runtime = super::super::core::MessageProcessingDelegate::ChatRuntime::new();
        runtime.activeTurnId = 2;
        runtime.isLoading = true;
        let error = runtime.reserveCancellation(Some(1)).unwrap_err();
        assert_eq!(error, "Work generation is no longer active");
        assert_eq!(runtime.activeTurnId, 2);
        assert!(runtime.isLoading);
        assert!(!runtime.isCancelling);
        assert_eq!(runtime.reserveCancellation(Some(2)).unwrap(), Some((2, 3)));
        assert!(runtime.isCancelling);
        assert!(runtime.reserveCancellation(Some(2)).is_err());
    }

    /// Verifies failed request ownership is released rather than leaving phantom running work.
    #[test]
    fn requestDropPublishesFailureWithoutResurrectingAnEndedRun() {
        let id = uuid::Uuid::new_v4().to_string();
        change(|state| {
            state
                .works
                .insert(id.clone(), work(&id, "test-drop-chat", 1));
        });
        drop(WorkRequestGuard::new(id.clone()));
        assert_eq!(hub().state.lock().unwrap().works[&id].status, "failed");
        let endedAt = hub().state.lock().unwrap().works[&id].endedAtMillis;
        drop(WorkRequestGuard::new(id.clone()));
        assert_eq!(
            hub().state.lock().unwrap().works[&id].endedAtMillis,
            endedAt
        );
    }
}
