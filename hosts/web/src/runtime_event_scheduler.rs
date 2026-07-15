use std::cell::RefCell;
use std::collections::BTreeMap;

use operit_host_api::{
    HostError, HostResult, HostRuntimeEventSchedule, HostRuntimeEventScheduleFire,
    HostRuntimeEventScheduleKind, HostRuntimeEventScheduleSink, HostRuntimeEventSchedulerHost,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

#[derive(Clone)]
struct ActiveSchedule {
    definition: HostRuntimeEventSchedule,
    nextFireAtMillis: u64,
}

#[derive(Default)]
struct WebSchedulerState {
    schedules: BTreeMap<String, ActiveSchedule>,
    sink: Option<HostRuntimeEventScheduleSink>,
    timeoutHandle: Option<i32>,
    timeoutClosure: Option<Closure<dyn FnMut()>>,
}

thread_local! {
    static WEB_SCHEDULER_STATE: RefCell<WebSchedulerState> = RefCell::new(WebSchedulerState::default());
}

/// Schedules ToolPkg timers and intervals with the browser event loop.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebHostRuntimeEventSchedulerHost;

impl WebHostRuntimeEventSchedulerHost {
    /// Creates the browser host event scheduler.
    pub fn new() -> Self {
        Self
    }
}

impl HostRuntimeEventSchedulerHost for WebHostRuntimeEventSchedulerHost {
    /// Reconciles browser timers without resetting unchanged schedule deadlines.
    fn replaceHostRuntimeEventSchedules(
        &self,
        schedules: Vec<HostRuntimeEventSchedule>,
        sink: HostRuntimeEventScheduleSink,
    ) -> HostResult<()> {
        let nowMillis = browserMillis();
        WEB_SCHEDULER_STATE.with(|slot| {
            let mut state = slot.borrow_mut();
            cancelTimeout(&mut state)?;
            let previous = std::mem::take(&mut state.schedules);
            let mut next = BTreeMap::new();
            for definition in schedules {
                validateSchedule(&definition)?;
                if next.contains_key(&definition.scheduleId) {
                    return Err(HostError::new(format!(
                        "duplicate browser host event scheduleId: {}",
                        definition.scheduleId
                    )));
                }
                let nextFireAtMillis = match previous.get(&definition.scheduleId) {
                    Some(active) if active.definition == definition => active.nextFireAtMillis,
                    _ => nowMillis.checked_add(definition.delayMs).ok_or_else(|| {
                        HostError::new(format!(
                            "browser host event schedule delay overflows epoch milliseconds: {}",
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
            armNextTimeout(&mut state)
        })
    }
}

/// Validates one browser timer or interval definition.
#[allow(non_snake_case)]
fn validateSchedule(schedule: &HostRuntimeEventSchedule) -> HostResult<()> {
    if schedule.scheduleId.is_empty() || schedule.delayMs == 0 {
        return Err(HostError::new(
            "browser host event schedule requires a non-empty id and positive delay",
        ));
    }
    match schedule.kind {
        HostRuntimeEventScheduleKind::Timer if schedule.intervalMs.is_some() => Err(
            HostError::new("browser timer schedule must not define intervalMs"),
        ),
        HostRuntimeEventScheduleKind::Interval
            if schedule.intervalMs.is_none() || schedule.intervalMs == Some(0) =>
        {
            Err(HostError::new(
                "browser interval schedule requires a positive intervalMs",
            ))
        }
        _ => Ok(()),
    }
}

/// Cancels the currently armed browser timeout.
#[allow(non_snake_case)]
fn cancelTimeout(state: &mut WebSchedulerState) -> HostResult<()> {
    if let Some(handle) = state.timeoutHandle.take() {
        browserWindow()?.clear_timeout_with_handle(handle);
    }
    state.timeoutClosure = None;
    Ok(())
}

/// Arms the browser timeout for the earliest outstanding schedule deadline.
#[allow(non_snake_case)]
fn armNextTimeout(state: &mut WebSchedulerState) -> HostResult<()> {
    let Some(nextFireAtMillis) = state
        .schedules
        .values()
        .map(|schedule| schedule.nextFireAtMillis)
        .min()
    else {
        return Ok(());
    };
    let remaining = nextFireAtMillis.saturating_sub(browserMillis());
    let delay = remaining.min(i32::MAX as u64) as i32;
    let callback = Closure::wrap(Box::new(processDueSchedules) as Box<dyn FnMut()>);
    let handle = browserWindow()?
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            delay,
        )
        .map_err(|error| {
            HostError::new(format!(
                "browser host event timeout registration failed: {error:?}"
            ))
        })?;
    state.timeoutHandle = Some(handle);
    state.timeoutClosure = Some(callback);
    Ok(())
}

/// Delivers all due browser schedules and advances repeating interval boundaries.
#[allow(non_snake_case)]
fn processDueSchedules() {
    let fires = WEB_SCHEDULER_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        state.timeoutHandle = None;
        state.timeoutClosure = None;
        let firedAtMillis = browserMillis();
        let dueIds = state
            .schedules
            .iter()
            .filter(|(_, schedule)| schedule.nextFireAtMillis <= firedAtMillis)
            .map(|(scheduleId, _)| scheduleId.clone())
            .collect::<Vec<_>>();
        let sink = state
            .sink
            .clone()
            .expect("browser host event schedule sink must be installed");
        let mut fires = Vec::with_capacity(dueIds.len());
        for scheduleId in dueIds {
            let active = state
                .schedules
                .get(&scheduleId)
                .expect("due browser schedule must exist")
                .clone();
            fires.push((
                sink.clone(),
                HostRuntimeEventScheduleFire {
                    scheduleId: scheduleId.clone(),
                    scheduledAtMillis: active.nextFireAtMillis,
                    firedAtMillis,
                },
            ));
            match active.definition.kind {
                HostRuntimeEventScheduleKind::Timer => {
                    state.schedules.remove(&scheduleId);
                }
                HostRuntimeEventScheduleKind::Interval => {
                    let intervalMs = active
                        .definition
                        .intervalMs
                        .expect("validated browser interval must define intervalMs");
                    state
                        .schedules
                        .get_mut(&scheduleId)
                        .expect("active browser interval must exist")
                        .nextFireAtMillis = nextIntervalTime(
                        active.nextFireAtMillis,
                        intervalMs,
                        firedAtMillis,
                    );
                }
            }
        }
        armNextTimeout(&mut state).expect("browser host event timeout must rearm");
        fires
    });
    for (sink, fire) in fires {
        sink(fire);
    }
}

/// Computes the next browser interval boundary without accumulating drift.
#[allow(non_snake_case)]
fn nextIntervalTime(scheduledAtMillis: u64, intervalMs: u64, nowMillis: u64) -> u64 {
    let elapsed = nowMillis.saturating_sub(scheduledAtMillis);
    let steps = elapsed / intervalMs + 1;
    scheduledAtMillis
        .checked_add(
            steps
                .checked_mul(intervalMs)
                .expect("validated browser interval steps must fit milliseconds"),
        )
        .expect("validated browser interval must fit epoch milliseconds")
}

/// Returns the active browser window used for scheduling.
#[allow(non_snake_case)]
fn browserWindow() -> HostResult<web_sys::Window> {
    web_sys::window().ok_or_else(|| HostError::new("browser window is unavailable"))
}

/// Returns the browser wall-clock timestamp in epoch milliseconds.
#[allow(non_snake_case)]
fn browserMillis() -> u64 {
    js_sys::Date::now() as u64
}
