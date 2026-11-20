//! Throttles real local-work counters without estimating compression time.
use crate::GithubReleaseUtil::{FullUpdateProgressEvent, UpdatePreparationPhase};
use std::sync::{Arc, Mutex};

pub(super) struct PreparationProgress {
    events: Arc<dyn Fn(FullUpdateProgressEvent) + Send + Sync>,
    latest: Mutex<Option<(UpdatePreparationPhase, u64, u64, u128)>>,
}

impl PreparationProgress {
    /// Shares one bounded progress stream across parallel Host operations.
    pub(super) fn new(events: Arc<dyn Fn(FullUpdateProgressEvent) + Send + Sync>) -> Self {
        Self {
            events,
            latest: Mutex::new(None),
        }
    }

    /// Emits phase transitions and completion immediately, with at most four intermediate events per second.
    pub(super) fn report(&self, phase: UpdatePreparationPhase, completed: u64, total: u64) {
        let now = operit_host_api::TimeUtils::currentTimeMillisU128();
        let mut latest = self.latest.lock().expect("update preparation progress");
        let completed = completed.min(total);
        if let Some((previous_phase, previous, previous_total, at)) = *latest {
            if phase == previous_phase && total == previous_total {
                if completed <= previous || (completed != total && now.saturating_sub(at) < 250) {
                    return;
                }
            }
        }
        *latest = Some((phase, completed, total, now));
        (self.events)(FullUpdateProgressEvent::PreparationProgress {
            phase,
            completedBytes: completed,
            totalBytes: total,
        });
    }
}
