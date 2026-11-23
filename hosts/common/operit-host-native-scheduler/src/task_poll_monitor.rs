//! Reports an executing poll that fails to yield, independently of its blocked executor.

use operit_host_api::{HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTaskSchedulerHost};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock, Weak},
    task::{Context, Poll},
    thread::ThreadId,
};

const ANR_THRESHOLD_MS: u64 = 2_000;
const SAMPLE_INTERVAL_MS: u64 = 100;

struct ActivePoll {
    started: u64,
    thread: ThreadId,
    reported: bool,
}

struct TaskProgress {
    name: String,
    active: Mutex<Option<ActivePoll>>,
}

impl TaskProgress {
    /// Publishes an executing poll without retaining a lock while application code runs.
    fn enter(self: &Arc<Self>) -> PollGuard {
        *self.active.lock().expect("task poll monitor mutex") = Some(ActivePoll {
            started: monotonicMillis(),
            thread: std::thread::current().id(),
            reported: false,
        });
        PollGuard(self.clone())
    }

    /// Claims one diagnostic per slow poll and optionally clears its execution state.
    fn sample(&self, now: u64, finished: bool) -> Option<String> {
        let mut active = self.active.lock().expect("task poll monitor mutex");
        let diagnostic = active.as_mut().and_then(|poll| {
            let elapsed = now.saturating_sub(poll.started);
            if poll.reported || elapsed < ANR_THRESHOLD_MS {
                return None;
            }
            poll.reported = true;
            Some((elapsed, poll.thread))
        });
        if finished {
            *active = None;
        }
        drop(active);
        diagnostic.map(|(elapsed, thread)| format!(
            "ANR task={} thread={thread:?} pollElapsedMs={elapsed} thresholdMs={ANR_THRESHOLD_MS} phase={}",
            self.name, if finished { "returned" } else { "polling" },
        ))
    }
}

struct PollGuard(Arc<TaskProgress>);

impl Drop for PollGuard {
    /// Clears Pending, Ready and unwinding polls; idle event waits are never classified as ANRs.
    fn drop(&mut self) {
        if let Some(message) = self.0.sample(monotonicMillis(), true) {
            operit_host_api::logHostError("HostRuntimeANR", &message);
        }
    }
}

pub(crate) struct MonitoredFuture<F: Future<Output = ()> + ?Sized> {
    source: Pin<Box<F>>,
    progress: Arc<TaskProgress>,
}

impl<F: Future<Output = ()> + ?Sized> Future for MonitoredFuture<F> {
    type Output = ();

    /// Times exactly one executor poll, including the task factory's synchronous work.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        let _guard = this.progress.enter();
        this.source.as_mut().poll(cx)
    }
}

/// Reads the Host clock shared with native scheduling and deadline diagnostics.
fn monotonicMillis() -> u64 {
    crate::NativeHostRuntimeTaskSchedulerHost
        .monotonicTimeMillis()
        .expect("native monotonic clock")
}

/// Starts one independent observer for monitored native coordination and blocking tasks.
fn registry() -> HostResult<&'static Arc<Mutex<Vec<Weak<TaskProgress>>>>> {
    type Registry = Arc<Mutex<Vec<Weak<TaskProgress>>>>;
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            let registry: Registry = Arc::new(Mutex::new(Vec::new()));
            let observed = registry.clone();
            std::thread::Builder::new()
                .name("operit-anr".into())
                .spawn(move || loop {
                    let tasks = {
                        let mut entries = observed.lock().expect("task monitor registry mutex");
                        let tasks: Vec<_> = entries.iter().filter_map(Weak::upgrade).collect();
                        entries.retain(|entry| entry.strong_count() != 0);
                        tasks
                    };
                    let now = monotonicMillis();
                    for task in tasks {
                        if let Some(message) = task.sample(now, false) {
                            operit_host_api::logHostError("HostRuntimeANR", &message);
                        }
                    }
                    std::thread::park_timeout(std::time::Duration::from_millis(SAMPLE_INTERVAL_MS));
                })
                .map_err(|error| error.to_string())?;
            Ok(registry)
        })
        .as_ref()
        .map_err(|error| HostError::new(format!("start task ANR monitor failed: {error}")))
}

/// Registers a named task without moving its thread-affine future between executors.
pub(crate) fn monitorTask(
    name: &str,
    task: HostRuntimeAsyncTask,
) -> HostResult<HostRuntimeAsyncTask> {
    let progress = register(name)?;
    Ok(Box::new(move || Box::pin(MonitoredFuture {
        source: Box::pin(async move { task().await }), progress,
    })))
}

/// Registers either affine or movable execution using the same observer and poll guard.
pub(crate) fn monitorFuture<F: Future<Output = ()> + ?Sized>(
    name: &str,
    source: Pin<Box<F>>,
) -> HostResult<Pin<Box<MonitoredFuture<F>>>> {
    Ok(Box::pin(MonitoredFuture { source, progress: register(name)? }))
}

/// Shares registration without constructing affine futures on the submitting thread.
fn register(name: &str) -> HostResult<Arc<TaskProgress>> {
    let progress = Arc::new(TaskProgress {
        name: name.into(),
        active: Mutex::new(None),
    });
    registry()?
        .lock()
        .expect("task monitor registry mutex")
        .push(Arc::downgrade(&progress));
    Ok(progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constructs an executing poll with a deterministic Host-clock timestamp.
    fn executing(started: u64) -> TaskProgress {
        TaskProgress {
            name: "test-watch".into(),
            active: Mutex::new(Some(ActivePoll {
                started,
                thread: std::thread::current().id(),
                reported: false,
            })),
        }
    }

    /// Reports a blocked poll at the threshold before it returns, once per poll.
    #[test]
    fn reports_an_executing_poll_once_at_two_seconds() {
        let task = executing(10);
        assert!(task.sample(2_009, false).is_none());
        let message = task.sample(2_010, false).unwrap();
        assert!(message.starts_with("ANR task=test-watch "));
        assert!(message.ends_with("phase=polling"));
        assert!(task.sample(4_010, false).is_none());
        assert!(task.sample(4_010, true).is_none());
        assert!(task.active.lock().unwrap().is_none());
    }

    /// Idle subscriptions never report an ANR, and a later poll starts a fresh interval.
    #[test]
    fn pending_waits_do_not_accumulate_execution_time() {
        let task = executing(10);
        assert!(task.sample(11, true).is_none());
        assert!(task.sample(100_000, false).is_none());
        *task.active.lock().unwrap() = executing(100_000).active.into_inner().unwrap();
        assert!(task.sample(100_001, true).is_none());
    }

    /// A slow poll returning between observer samples still produces a diagnostic.
    #[test]
    fn reports_a_slow_return_without_waiting_for_an_observer_tick() {
        let task = executing(10);
        assert!(task
            .sample(2_010, true)
            .unwrap()
            .ends_with("phase=returned"));
        assert!(task.sample(5_000, false).is_none());
    }
}
