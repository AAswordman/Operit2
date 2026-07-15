use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use operit_host_api::{
    HostError, HostResult, HostRuntimeEventSchedule, HostRuntimeEventScheduleFire,
    HostRuntimeEventScheduleKind, HostRuntimeEventScheduleSink, HostRuntimeEventSchedulerHost,
};

#[derive(Clone)]
struct ActiveSchedule {
    definition: HostRuntimeEventSchedule,
    nextFireAtMillis: u64,
}

#[derive(Default)]
struct SchedulerState {
    schedules: BTreeMap<String, ActiveSchedule>,
    sink: Option<HostRuntimeEventScheduleSink>,
    stopped: bool,
}

struct SchedulerShared {
    state: Mutex<SchedulerState>,
    changed: Condvar,
}

impl Drop for SchedulerShared {
    /// Stops the coordinator when the final host scheduler reference is released.
    fn drop(&mut self) {
        self.state
            .get_mut()
            .expect("native event scheduler state lock must not be poisoned")
            .stopped = true;
        self.changed.notify_all();
    }
}

/// Schedules host events on one native process coordinator thread.
#[derive(Clone)]
pub struct NativeHostRuntimeEventSchedulerHost {
    shared: Arc<SchedulerShared>,
}

impl NativeHostRuntimeEventSchedulerHost {
    /// Creates and starts the native host event schedule coordinator.
    pub fn new() -> Self {
        let shared = Arc::new(SchedulerShared {
            state: Mutex::new(SchedulerState::default()),
            changed: Condvar::new(),
        });
        let workerShared = Arc::downgrade(&shared);
        thread::Builder::new()
            .name("operit-host-event-scheduler".to_string())
            .spawn(move || runScheduler(workerShared))
            .expect("native host event scheduler thread must start");
        Self { shared }
    }
}

impl Default for NativeHostRuntimeEventSchedulerHost {
    /// Creates the default native host event scheduler.
    fn default() -> Self {
        Self::new()
    }
}

impl HostRuntimeEventSchedulerHost for NativeHostRuntimeEventSchedulerHost {
    /// Reconciles the complete native process schedule set without resetting unchanged timers.
    fn replaceHostRuntimeEventSchedules(
        &self,
        schedules: Vec<HostRuntimeEventSchedule>,
        sink: HostRuntimeEventScheduleSink,
    ) -> HostResult<()> {
        let nowMillis = unixMillis();
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| HostError::new("native host event scheduler lock is poisoned"))?;
        let previous = std::mem::take(&mut state.schedules);
        let mut next = BTreeMap::new();
        for definition in schedules {
            if next.contains_key(&definition.scheduleId) {
                return Err(HostError::new(format!(
                    "duplicate host event scheduleId: {}",
                    definition.scheduleId
                )));
            }
            validateSchedule(&definition)?;
            let nextFireAtMillis = match previous.get(&definition.scheduleId) {
                Some(active) if active.definition == definition => active.nextFireAtMillis,
                _ => nowMillis.checked_add(definition.delayMs).ok_or_else(|| {
                    HostError::new(format!(
                        "host event schedule delay overflows epoch milliseconds: {}",
                        definition.scheduleId
                    ))
                })?,
            };
            next.insert(
                definition.scheduleId.clone(),
                ActiveSchedule {
                    definition,
                    nextFireAtMillis,
                },
            );
        }
        state.schedules = next;
        state.sink = Some(sink);
        drop(state);
        self.shared.changed.notify_all();
        Ok(())
    }
}

/// Validates one native timer or interval definition before it reaches the worker.
#[allow(non_snake_case)]
fn validateSchedule(schedule: &HostRuntimeEventSchedule) -> HostResult<()> {
    if schedule.scheduleId.is_empty() {
        return Err(HostError::new("host event scheduleId is required"));
    }
    if schedule.delayMs == 0 {
        return Err(HostError::new(format!(
            "host event schedule delayMs must be positive: {}",
            schedule.scheduleId
        )));
    }
    match schedule.kind {
        HostRuntimeEventScheduleKind::Timer if schedule.intervalMs.is_some() => {
            Err(HostError::new(format!(
                "timer schedule must not define intervalMs: {}",
                schedule.scheduleId
            )))
        }
        HostRuntimeEventScheduleKind::Interval if schedule.intervalMs == Some(0) => {
            Err(HostError::new(format!(
                "interval schedule intervalMs must be positive: {}",
                schedule.scheduleId
            )))
        }
        HostRuntimeEventScheduleKind::Interval if schedule.intervalMs.is_none() => {
            Err(HostError::new(format!(
                "interval schedule requires intervalMs: {}",
                schedule.scheduleId
            )))
        }
        _ => Ok(()),
    }
}

/// Runs the single native schedule coordinator until its owning host is released.
#[allow(non_snake_case)]
fn runScheduler(shared: Weak<SchedulerShared>) {
    loop {
        let Some(shared) = shared.upgrade() else {
            return;
        };
        let mut state = shared
            .state
            .lock()
            .expect("native host event scheduler lock must not be poisoned");
        if state.stopped {
            return;
        }
        let Some(nextFireAtMillis) = state
            .schedules
            .values()
            .map(|schedule| schedule.nextFireAtMillis)
            .min()
        else {
            drop(
                shared
                    .changed
                    .wait(state)
                    .expect("native host event scheduler lock must not be poisoned"),
            );
            continue;
        };
        let nowMillis = unixMillis();
        if nextFireAtMillis > nowMillis {
            let waitDuration = Duration::from_millis(nextFireAtMillis - nowMillis);
            drop(
                shared
                    .changed
                    .wait_timeout(state, waitDuration)
                    .expect("native host event scheduler lock must not be poisoned"),
            );
            continue;
        }
        let firedAtMillis = unixMillis();
        let dueIds = state
            .schedules
            .iter()
            .filter(|(_, schedule)| schedule.nextFireAtMillis <= firedAtMillis)
            .map(|(scheduleId, _)| scheduleId.clone())
            .collect::<Vec<_>>();
        let sink = state
            .sink
            .clone()
            .expect("native host event schedule sink must be installed");
        let mut fires = Vec::with_capacity(dueIds.len());
        for scheduleId in dueIds {
            let active = state
                .schedules
                .get(&scheduleId)
                .expect("due native host event schedule must exist")
                .clone();
            fires.push(HostRuntimeEventScheduleFire {
                scheduleId: scheduleId.clone(),
                scheduledAtMillis: active.nextFireAtMillis,
                firedAtMillis,
            });
            match active.definition.kind {
                HostRuntimeEventScheduleKind::Timer => {
                    state.schedules.remove(&scheduleId);
                }
                HostRuntimeEventScheduleKind::Interval => {
                    let intervalMs = active
                        .definition
                        .intervalMs
                        .expect("validated interval schedule must define intervalMs");
                    let nextFireAtMillis =
                        nextIntervalTime(active.nextFireAtMillis, intervalMs, firedAtMillis);
                    state
                        .schedules
                        .get_mut(&scheduleId)
                        .expect("active interval schedule must exist")
                        .nextFireAtMillis = nextFireAtMillis;
                }
            }
        }
        drop(state);
        drop(shared);
        for fire in fires {
            sink(fire);
        }
    }
}

/// Computes the next future interval boundary without accumulating drift.
#[allow(non_snake_case)]
fn nextIntervalTime(scheduledAtMillis: u64, intervalMs: u64, nowMillis: u64) -> u64 {
    if scheduledAtMillis > nowMillis {
        return scheduledAtMillis
            .checked_add(intervalMs)
            .expect("validated interval must fit epoch milliseconds");
    }
    let elapsed = nowMillis - scheduledAtMillis;
    let steps = elapsed / intervalMs + 1;
    scheduledAtMillis
        .checked_add(
            steps
                .checked_mul(intervalMs)
                .expect("validated interval steps must fit milliseconds"),
        )
        .expect("validated interval must fit epoch milliseconds")
}

/// Returns the current Unix epoch time in milliseconds.
#[allow(non_snake_case)]
fn unixMillis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after Unix epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// Verifies timer and interval firings preserve identity and platform schedule timestamps.
    #[test]
    fn fires_timer_and_interval_from_one_coordinator() {
        let host = NativeHostRuntimeEventSchedulerHost::new();
        let (sender, receiver) = mpsc::channel();
        host.replaceHostRuntimeEventSchedules(
            vec![
                HostRuntimeEventSchedule {
                    scheduleId: "timer".to_string(),
                    containerPackageName: "example".to_string(),
                    hookId: "once".to_string(),
                    kind: HostRuntimeEventScheduleKind::Timer,
                    delayMs: 10,
                    intervalMs: None,
                },
                HostRuntimeEventSchedule {
                    scheduleId: "interval".to_string(),
                    containerPackageName: "example".to_string(),
                    hookId: "repeat".to_string(),
                    kind: HostRuntimeEventScheduleKind::Interval,
                    delayMs: 10,
                    intervalMs: Some(10),
                },
            ],
            Arc::new(move |fire| {
                sender.send(fire).expect("test receiver must remain open");
            }),
        )
        .expect("valid schedules must install");

        let first = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("first schedule must fire");
        let second = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("second schedule must fire");
        let ids = [first.scheduleId.as_str(), second.scheduleId.as_str()];
        assert!(ids.contains(&"timer"));
        assert!(ids.contains(&"interval"));
        assert!(first.firedAtMillis >= first.scheduledAtMillis);
        assert!(second.firedAtMillis >= second.scheduledAtMillis);

        let repeated = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("interval schedule must repeat");
        assert_eq!(repeated.scheduleId, "interval");
    }
}
