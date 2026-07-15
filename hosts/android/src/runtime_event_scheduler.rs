use std::sync::{Mutex, OnceLock};

use operit_host_api::{
    HostError, HostResult, HostRuntimeEventSchedule, HostRuntimeEventScheduleFire,
    HostRuntimeEventScheduleSink, HostRuntimeEventSchedulerHost,
};

use crate::secret_store::androidHostSecretStoreBridge;

/// Owns Android system timer and interval reconciliation for host events.
#[derive(Clone, Debug, Default)]
pub struct AndroidHostRuntimeEventSchedulerHost;

impl AndroidHostRuntimeEventSchedulerHost {
    /// Creates the Android host event scheduler.
    pub fn new() -> Self {
        Self
    }
}

/// Returns the process-wide schedule firing sink installed by Core.
fn scheduleSinkSlot() -> &'static Mutex<Option<HostRuntimeEventScheduleSink>> {
    static SINK: OnceLock<Mutex<Option<HostRuntimeEventScheduleSink>>> = OnceLock::new();
    SINK.get_or_init(|| Mutex::new(None))
}

impl HostRuntimeEventSchedulerHost for AndroidHostRuntimeEventSchedulerHost {
    /// Replaces Android AlarmManager schedules and installs their Core callback.
    fn replaceHostRuntimeEventSchedules(
        &self,
        schedules: Vec<HostRuntimeEventSchedule>,
        sink: HostRuntimeEventScheduleSink,
    ) -> HostResult<()> {
        *scheduleSinkSlot()
            .lock()
            .map_err(|_| HostError::new("Android runtime event schedule sink lock is poisoned"))? =
            Some(sink);
        androidHostSecretStoreBridge()?.replaceHostRuntimeEventSchedules(&schedules)
    }
}

/// Delivers one Android AlarmManager firing into the registered Core runtime.
pub fn emitAndroidHostRuntimeEventSchedule(fire: HostRuntimeEventScheduleFire) -> HostResult<()> {
    let sink = scheduleSinkSlot()
        .lock()
        .map_err(|_| HostError::new("Android runtime event schedule sink lock is poisoned"))?
        .clone()
        .ok_or_else(|| HostError::new("Android runtime event schedule sink is not registered"))?;
    sink(fire);
    Ok(())
}

/// Clears the Core callback used by Android system schedule firings.
pub(crate) fn clearAndroidHostRuntimeEventScheduleSink() {
    *scheduleSinkSlot()
        .lock()
        .expect("Android runtime event schedule sink lock must not be poisoned") = None;
}
