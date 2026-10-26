//! Native offline ABI; all target selection remains behind platform_runtime.
use operit_host_data_rescue::{NativeRescueSession, RescueProgress, StorageLease};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static RUNTIME_LEASE: OnceLock<Mutex<Option<StorageLease>>> = OnceLock::new();
static RESCUE_SESSION: OnceLock<Mutex<Option<NativeRescueSession>>> = OnceLock::new();

static JOB: Mutex<Option<RescueJob>> = Mutex::new(None);
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);

/// Retains one task result independently of the long-running session lock.
struct RescueJob {
    id: u64,
    progress: Arc<Mutex<Option<RescueProgress>>>,
    result: Arc<Mutex<Option<Result<Value, String>>>>,
    worker: operit_host_native_common::NativeTaskHandle<()>,
}

/// Starts one background operation while keeping platform request queues available.
fn start_job(default_runtime: PathBuf, request: Value) -> Result<Value, String> {
    let operation = request["request"].clone();
    match operation["operation"].as_str() {
        Some(
            "export" | "inspect" | "restore" | "resumeRestore" | "saveArchive"
            | "configureAutomatic",
        ) => (),
        _ => return Err("Unsupported background maintenance operation".into()),
    }
    let mut jobs = JOB.lock().map_err(|error| error.to_string())?;
    if jobs.is_some() {
        return Err("A maintenance job is already active".into());
    }
    let session = RESCUE_SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|error| error.to_string())?;
    let progress = session
        .as_ref()
        .ok_or("Open an exclusive maintenance session first")?
        .progress();
    *progress.lock().map_err(|error| error.to_string())? = None;
    drop(session);
    let id = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
    let result = Arc::new(Mutex::new(None));
    let completion = result.clone();
    let worker = operit_host_native_common::scheduleNativeBlockingTask("data-rescue", move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dispatch_session(default_runtime, operation)
        }))
        .map_err(|_| "Maintenance worker panicked".to_string())
        .and_then(|value| value);
        *completion.lock().expect("Rescue job result lock poisoned") = Some(outcome);
    })
    .map_err(|error| error.to_string())?;
    *jobs = Some(RescueJob {
        id,
        progress,
        result,
        worker,
    });
    Ok(json!({"jobId": id}))
}

/// Reads the latest measured stage without waiting for filesystem work.
fn poll_job(request: &Value) -> Result<Value, String> {
    let mut jobs = JOB.lock().map_err(|error| error.to_string())?;
    let job = jobs.as_ref().ok_or("No maintenance job is active")?;
    if request["jobId"].as_u64() != Some(job.id) {
        return Err("Unknown maintenance job".into());
    }
    let progress = job
        .progress
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    let result = job.result.lock().map_err(|error| error.to_string())?.take();
    match result {
        None => Ok(json!({"done": false, "progress": progress})),
        Some(result) => {
            *jobs = None;
            match result {
                Ok(value) => Ok(json!({"done": true, "progress": progress, "value": value})),
                Err(error) => Err(error),
            }
        }
    }
}

/// Executes an offline maintenance command and preserves structured diagnostics at the ABI boundary.
pub fn execute(default_runtime: PathBuf, encoded: &str) -> String {
    let result = serde_json::from_str(encoded)
        .map_err(|error| error.to_string())
        .and_then(|request| dispatch(default_runtime, request));
    match result {
        Ok(value) => json!({"ok": true, "value": value}).to_string(),
        Err(error) => json!({"ok": false, "error": error}).to_string(),
    }
}

/// Waits for accepted maintenance work to finish before releasing the process-owned exclusive session.
fn close_session() -> Result<Value, String> {
    let mut jobs = JOB.lock().map_err(|error| error.to_string())?;
    let completion = match jobs.take() {
        Some(job) => job
            .worker
            .join()
            .map_err(|_| "Maintenance worker failed while closing its session".to_string()),
        None => Ok(()),
    };
    let mut session = RESCUE_SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|error| error.to_string())?;
    *session = None;
    completion?;
    Ok(Value::Null)
}

/// Separates process access leases from exclusive rescue sessions before touching any business state.
fn dispatch(default_runtime: PathBuf, request: Value) -> Result<Value, String> {
    if request["operation"] == "launchMode" {
        return Ok(json!({"rescue": false}));
    }
    if request["operation"] == "runtimeLease" {
        let mut lease = RUNTIME_LEASE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|error| error.to_string())?;
        if lease.is_none() {
            *lease = Some(StorageLease::acquire(&default_runtime, false)?);
        }
        return Ok(Value::Null);
    }
    if request["operation"] == "stopNormalProcess" {
        return Err(
            "The current platform does not expose a process-stop operation through the rescue ABI"
                .into(),
        );
    }
    if request["operation"] == "close" {
        return close_session();
    }
    if request["operation"] == "start" {
        return start_job(default_runtime, request);
    }
    if request["operation"] == "poll" {
        return poll_job(&request);
    }
    if JOB.lock().map_err(|error| error.to_string())?.is_some() {
        return Err("A maintenance job is already active".into());
    }
    dispatch_session(default_runtime, request)
}

/// Executes one exclusive operation on the session owned by the background task.
fn dispatch_session(default_runtime: PathBuf, request: Value) -> Result<Value, String> {
    let mut session = RESCUE_SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|error| error.to_string())?;
    match request["operation"].as_str() {
        Some("open") => {
            if session.is_none() {
                let secrets = crate::PlatformRuntimeFactory::rescue_secret_store(&default_runtime)?;
                *session = Some(NativeRescueSession::open(default_runtime, secrets)?);
            }
            session
                .as_ref()
                .ok_or("Maintenance session is unavailable")?
                .describe_startup()
        }
        _ => session
            .as_mut()
            .ok_or("Open an exclusive maintenance session first")?
            .execute(request),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// Reads progress while the session is locked and consumes terminal results exactly once.
    #[test]
    fn polling_is_independent_of_storage_and_preserves_task_failures() {
        let _serial = TEST_MUTEX.lock().unwrap();
        let _session = RESCUE_SESSION
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap();
        let progress = Arc::new(Mutex::new(Some(RescueProgress::new(
            "restore",
            Some(1024),
            "bytes",
        ))));
        let result = Arc::new(Mutex::new(None));
        *JOB.lock().unwrap() = Some(RescueJob {
            id: 42,
            progress: progress.clone(),
            result: result.clone(),
            worker: operit_host_native_common::scheduleNativeBlockingTask("rescue-test", || {})
                .unwrap(),
        });
        assert!(poll_job(&json!({"jobId": 43})).is_err());
        let first = poll_job(&json!({"jobId": 42})).unwrap();
        assert_eq!(first["done"], false);
        assert_eq!(first["progress"]["completed"], 0);
        progress.lock().unwrap().as_mut().unwrap().completed = 512;
        assert_eq!(
            poll_job(&json!({"jobId": 42})).unwrap()["progress"]["completed"],
            512
        );
        *result.lock().unwrap() = Some(Err("injected integrity failure".into()));
        assert_eq!(
            poll_job(&json!({"jobId": 42})).unwrap_err(),
            "injected integrity failure"
        );
        assert!(JOB.lock().unwrap().is_none());
        assert!(poll_job(&json!({"jobId": 42})).is_err());
    }

    /// Keeps the exclusive lease until accepted work finishes, then permits normal startup without a process restart.
    #[test]
    fn closing_maintenance_waits_for_worker_and_releases_storage() {
        let _serial = TEST_MUTEX.lock().unwrap();
        let root =
            std::env::temp_dir().join(format!("operit-rescue-close-{}", uuid::Uuid::new_v4()));
        let runtime = root.join("runtime");
        let session = NativeRescueSession::open(runtime.clone(), None).unwrap();
        *RESCUE_SESSION
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = Some(session);
        let (ready_sender, ready) = std::sync::mpsc::channel();
        let (finish, finished) = std::sync::mpsc::channel();
        let result = Arc::new(Mutex::new(None));
        let worker_result = result.clone();
        let worker =
            operit_host_native_common::scheduleNativeBlockingTask("rescue-test", move || {
                let _session = RESCUE_SESSION.get().unwrap().lock().unwrap();
                ready_sender.send(()).unwrap();
                finished.recv().unwrap();
                *worker_result.lock().unwrap() = Some(Ok(Value::Null));
            })
            .unwrap();
        *JOB.lock().unwrap() = Some(RescueJob {
            id: 99,
            progress: Arc::new(Mutex::new(None)),
            result,
            worker,
        });
        ready.recv().unwrap();
        let closing = std::thread::spawn(close_session);
        assert!(StorageLease::acquire(&runtime, false).is_err());
        finish.send(()).unwrap();
        assert_eq!(closing.join().unwrap().unwrap(), Value::Null);
        assert!(JOB.lock().unwrap().is_none());
        assert!(RESCUE_SESSION.get().unwrap().lock().unwrap().is_none());
        let runtime_lease = StorageLease::acquire(&runtime, false).unwrap();
        assert_eq!(close_session().unwrap(), Value::Null);
        drop(runtime_lease);
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Releases an idle maintenance lease even when a completed worker reports a panic.
    #[test]
    fn closing_maintenance_releases_storage_after_worker_failure() {
        let _serial = TEST_MUTEX.lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "operit-rescue-close-failure-{}",
            uuid::Uuid::new_v4()
        ));
        let runtime = root.join("runtime");
        *RESCUE_SESSION
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = Some(NativeRescueSession::open(runtime.clone(), None).unwrap());
        let worker = operit_host_native_common::scheduleNativeBlockingTask("rescue-test", || {
            panic!("injected maintenance worker failure")
        })
        .unwrap();
        *JOB.lock().unwrap() = Some(RescueJob {
            id: 100,
            progress: Arc::new(Mutex::new(None)),
            result: Arc::new(Mutex::new(None)),
            worker,
        });
        assert!(close_session().is_err());
        let runtime_lease = StorageLease::acquire(&runtime, false).unwrap();
        drop(runtime_lease);
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
