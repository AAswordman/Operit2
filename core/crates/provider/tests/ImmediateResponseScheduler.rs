//! Shared Host scheduler for integration fixtures that return responses without retrying.

use operit_host_api::{
    HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};

struct Scheduler(std::time::Instant);

/// Installs one scheduler in each integration test process before collecting model responses.
pub fn install() {
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(std::sync::Arc::new(
        Scheduler(std::time::Instant::now()),
    ));
}

impl HostRuntimeTaskSchedulerHost for Scheduler {
    /// Reads elapsed fixture time for runtime diagnostics.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(self.0.elapsed().as_millis() as u64)
    }

    /// Executes synchronous fixture work on a separate thread.
    fn scheduleHostRuntimeTask(&self, _: &str, task: HostRuntimeTask) -> HostResult<()> {
        std::thread::spawn(task);
        Ok(())
    }

    /// Creates each local provider future on the executor that owns it.
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(task());
        });
        Ok(())
    }

    /// Uses the same executor policy for cooperative fixture tasks.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, task)
    }

    /// Rejects unexpected delayed task dispatch in immediate-response fixtures.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        panic!("immediate-response fixture must not schedule delayed tasks")
    }

    /// Provides an immediate event-loop turn for deterministic fixtures.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }

    /// Keeps first-output deadlines pending and rejects unexpected retry delays.
    fn waitForHostRuntimeDelay(&self, delay_ms: u64) -> HostRuntimeTurnFuture {
        assert_eq!(
            delay_ms, 120_000,
            "immediate-response fixture must not retry"
        );
        Box::pin(std::future::pending())
    }
}
