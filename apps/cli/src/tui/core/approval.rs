use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use operit_runtime::api::chat::enhance::ToolExecutionManager::AITool;
use operit_runtime::core::tools::ToolPermissionSystem::PermissionRequestResult;

const PERMISSION_REQUEST_TIMEOUT_MS: u64 = 60_000;

#[derive(Clone)]
pub(super) struct TuiApprovalBridge {
    inner: Arc<ApprovalInner>,
}

struct ApprovalInner {
    state: Mutex<ApprovalState>,
    changed: Condvar,
}

#[derive(Clone, Debug)]
pub(super) struct PendingApproval {
    pub(super) tool: AITool,
    pub(super) description: String,
    pub(super) requested_at: Instant,
}

#[derive(Debug)]
struct ApprovalState {
    pending: Option<PendingApproval>,
    response: Option<PermissionRequestResult>,
}

impl TuiApprovalBridge {
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(ApprovalInner {
                state: Mutex::new(ApprovalState {
                    pending: None,
                    response: None,
                }),
                changed: Condvar::new(),
            }),
        }
    }

    pub(super) fn request(&self, tool: &AITool, description: &str) -> PermissionRequestResult {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("approval state mutex poisoned");
        state.pending = Some(PendingApproval {
            tool: tool.clone(),
            description: description.to_string(),
            requested_at: Instant::now(),
        });
        state.response = None;
        self.inner.changed.notify_all();

        let timeout = Duration::from_millis(PERMISSION_REQUEST_TIMEOUT_MS);
        let started_at = Instant::now();
        loop {
            if let Some(response) = state.response.take() {
                state.pending = None;
                self.inner.changed.notify_all();
                return response;
            }
            let elapsed = started_at.elapsed();
            if elapsed >= timeout {
                state.pending = None;
                self.inner.changed.notify_all();
                return PermissionRequestResult::DENY;
            }
            let wait = timeout.saturating_sub(elapsed);
            let (next_state, result) = self
                .inner
                .changed
                .wait_timeout(state, wait)
                .expect("approval state mutex poisoned");
            state = next_state;
            if result.timed_out() {
                state.pending = None;
                self.inner.changed.notify_all();
                return PermissionRequestResult::DENY;
            }
        }
    }

    pub(super) fn current(&self) -> Option<PendingApproval> {
        self.inner
            .state
            .lock()
            .expect("approval state mutex poisoned")
            .pending
            .clone()
    }

    pub(super) fn respond(&self, response: PermissionRequestResult) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("approval state mutex poisoned");
        if state.pending.is_some() {
            state.response = Some(response);
            self.inner.changed.notify_all();
        }
    }
}
