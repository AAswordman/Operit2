//! Joinable results for work allocated by the native Host scheduler.
use crate::NativeHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostResult, HostRuntimeTaskSchedulerHost};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

/// Retains a task result without owning or creating an operating-system thread.
pub struct NativeTaskHandle<T> {
    result: mpsc::Receiver<thread::Result<T>>,
    completed: Arc<AtomicBool>,
}

impl<T> NativeTaskHandle<T> {
    /// Waits for this task's result while leaving its reusable Host worker alive.
    pub fn join(self) -> thread::Result<T> {
        self.result
            .recv()
            .map_err(|error| Box::new(error) as Box<dyn std::any::Any + Send>)?
    }

    /// Bounds a shutdown wait without creating a separate thread to join this task.
    pub fn join_timeout(
        self,
        timeout: std::time::Duration,
    ) -> Result<thread::Result<T>, mpsc::RecvTimeoutError> {
        self.result.recv_timeout(timeout)
    }

    /// Reports completion independently of the lifetime of the allocated worker.
    pub fn is_finished(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }
}

/// Allocates one persistent blocking OS service from the Host's bounded service capacity.
pub fn scheduleNativeServiceTask<T: Send + 'static>(
    name: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> HostResult<NativeTaskHandle<T>> {
    let (sender, result) = mpsc::sync_channel(1);
    let completed = Arc::new(AtomicBool::new(false));
    let finished = completed.clone();
    NativeHostRuntimeTaskSchedulerHost::new().scheduleServiceTask(
        name,
        Box::new(move || {
            let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
            finished.store(true, Ordering::Release);
            let _ = sender.send(output);
        }),
    )?;
    Ok(NativeTaskHandle { result, completed })
}

/// Allocates finite blocking work from the same Host pool used by runtime blocking tasks.
pub fn scheduleNativeBlockingTask<T: Send + 'static>(
    name: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> HostResult<NativeTaskHandle<T>> {
    let (sender, result) = mpsc::sync_channel(1);
    let completed = Arc::new(AtomicBool::new(false));
    let finished = completed.clone();
    NativeHostRuntimeTaskSchedulerHost::new().scheduleHostRuntimeTask(
        name,
        Box::new(move || {
            let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
            finished.store(true, Ordering::Release);
            let _ = sender.send(output);
        }),
    )?;
    Ok(NativeTaskHandle { result, completed })
}

/// Returns joinable results after atomically admitting every component of an OS service group.
pub fn scheduleNativeServiceTasks(
    name: &str,
    work: Vec<operit_host_api::HostRuntimeTask>,
) -> HostResult<Vec<NativeTaskHandle<()>>> {
    let mut handles = Vec::with_capacity(work.len());
    let tasks = work
        .into_iter()
        .map(|work| {
            let (sender, result) = mpsc::sync_channel(1);
            let completed = Arc::new(AtomicBool::new(false));
            let finished = completed.clone();
            handles.push(NativeTaskHandle { result, completed });
            Box::new(move || {
                let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
                finished.store(true, Ordering::Release);
                let _ = sender.send(output);
            }) as operit_host_api::HostRuntimeTask
        })
        .collect();
    NativeHostRuntimeTaskSchedulerHost::new().scheduleServiceTasks(name, tasks)?;
    Ok(handles)
}
