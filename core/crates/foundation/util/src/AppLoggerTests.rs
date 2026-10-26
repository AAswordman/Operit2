use super::*;
use operit_host_api::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Barrier, Condvar};
use std::time::{Duration, Instant};

/// Serializes tests that bind the process-global AppLogger state.
static GLOBAL_LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Locks the shared logger fixture for one test.
fn global_log_guard() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_LOG_TEST_LOCK
        .lock()
        .expect("AppLogger test mutex poisoned")
}

/// Gates appends so a test can park one writer inside the host file write.
struct WriteGate {
    state: Mutex<(bool, bool)>, // (entered, proceed)
    cv: Condvar,
}

impl WriteGate {
    /// Creates a closed writer gate.
    fn new() -> Self {
        Self {
            state: Mutex::new((false, false)),
            cv: Condvar::new(),
        }
    }

    /// Called by the gated writer: announce entry, then wait for release.
    fn wait_until_released(&self) {
        let mut state = self.state.lock().unwrap();
        state.0 = true;
        self.cv.notify_all();
        while !state.1 {
            state = self.cv.wait(state).unwrap();
        }
    }

    /// Waits until a writer enters the gate; false if none arrives in time.
    fn wait_entered(&self, timeout: Duration) -> bool {
        let mut state = self.state.lock().unwrap();
        let deadline = Instant::now() + timeout;
        while !state.0 {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let (guard, _timed_out) = self.cv.wait_timeout(state, deadline - now).unwrap();
            state = guard;
        }
        true
    }

    /// Releases the parked writer.
    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.1 = true;
        self.cv.notify_all();
    }
}

#[derive(Default)]
struct TestFiles {
    files: HashMap<String, String>,
    directories: HashSet<String>,
}

#[derive(Default)]
struct TestFileSystemHost {
    storage: Mutex<TestFiles>,
    fail_inspect: bool,
    fail_directory: bool,
    fail_write: Option<String>,
    append_count: std::sync::atomic::AtomicUsize,
    fail_move: Option<String>,
    write_gate: Option<Arc<WriteGate>>,
    move_gate: Option<Arc<WriteGate>>,
}

impl FileSystemHost for TestFileSystemHost {
    /// Returns the test host label.
    fn envLabel(&self) -> &str {
        "test"
    }

    /// Describes the in-memory test host.
    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::linux()
    }

    /// Inspects the in-memory files and directories.
    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        if self.fail_inspect {
            return Err(HostError::new("Permission denied inspecting file"));
        }
        let storage = self.storage.lock().unwrap();
        let is_directory = storage.directories.contains(path);
        let file = storage.files.get(path);
        Ok(FileExistence {
            exists: is_directory || file.is_some(),
            isDirectory: is_directory,
            size: file.map_or(0, |content| content.len() as i64),
        })
    }

    /// Creates the test log directory or reports the injected failure.
    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        assert!(
            createParents,
            "first-launch logs require recursive creation"
        );
        if self.fail_directory {
            return Err(HostError::new("Permission denied creating directory"));
        }
        self.storage.lock().unwrap().directories.insert(path.into());
        Ok(())
    }

    /// Writes test records and applies configured writer gates.
    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        if self.fail_write.as_deref() == Some(path) {
            return Err(HostError::new("Permission denied writing file"));
        }
        if append {
            if let Some(gate) = &self.write_gate {
                gate.wait_until_released();
            }
        }
        let parent = std::path::Path::new(path).parent().unwrap();
        let mut storage = self.storage.lock().unwrap();
        if !storage.directories.contains(parent.to_str().unwrap()) {
            return Err(HostError::new("Parent directory does not exist"));
        }
        if append {
            self.append_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            storage
                .files
                .entry(path.into())
                .or_default()
                .push_str(content);
        } else {
            storage.files.insert(path.into(), content.into());
        }
        Ok(())
    }

    /// Implements the validatePath operation for the logger.
    fn validatePath(&self, _path: &str, _paramName: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the listFiles operation for the logger.
    fn listFiles(&self, _path: &str) -> HostResult<Vec<FileEntry>> {
        unreachable!("unrelated file-system operation")
    }

    /// Returns the requested in-memory log contents.
    fn readFile(&self, path: &str) -> HostResult<String> {
        self.storage
            .lock()
            .unwrap()
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| HostError::new("File does not exist"))
    }

    /// Implements the readFileWithLimit operation for the logger.
    fn readFileWithLimit(&self, _path: &str, _maxBytes: usize) -> HostResult<String> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the readFileBytes operation for the logger.
    fn readFileBytes(&self, _path: &str) -> HostResult<Vec<u8>> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the writeFileBytes operation for the logger.
    fn writeFileBytes(&self, _path: &str, _content: &[u8]) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    /// Removes one in-memory archive.
    fn deleteFile(&self, path: &str, _recursive: bool) -> HostResult<()> {
        let mut storage = self.storage.lock().unwrap();
        if storage.files.remove(path).is_some() {
            Ok(())
        } else {
            Err(HostError::new(format!("No such file: {path}")))
        }
    }

    /// Moves a log into its archive while applying the rotation gate.
    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        if self.fail_move.as_deref() == Some(source) {
            return Err(HostError::new("Permission denied moving file"));
        }
        let content = self
            .storage
            .lock()
            .unwrap()
            .files
            .remove(source)
            .ok_or_else(|| HostError::new(format!("No such file: {source}")))?;
        if let Some(gate) = &self.move_gate {
            gate.wait_until_released();
        }
        self.storage
            .lock()
            .unwrap()
            .files
            .insert(destination.into(), content);
        Ok(())
    }

    /// Implements the copyFile operation for the logger.
    fn copyFile(&self, _source: &str, _destination: &str, _recursive: bool) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the findFiles operation for the logger.
    fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
        unreachable!("unrelated file-system operation")
    }

    /// Returns the in-memory log metadata.
    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        let storage = self.storage.lock().unwrap();
        let size = storage.files.get(path).map(|content| content.len() as i64);
        Ok(FileInfo {
            path: path.into(),
            exists: size.is_some(),
            fileType: "file".into(),
            size: size.unwrap_or(0),
            permissions: "rw".into(),
            owner: String::new(),
            group: String::new(),
            lastModified: String::new(),
            rawStatOutput: String::new(),
        })
    }

    /// Implements the grepCode operation for the logger.
    fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the zipFiles operation for the logger.
    fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the unzipFiles operation for the logger.
    fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the openFile operation for the logger.
    fn openFile(&self, _path: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    /// Implements the shareFile operation for the logger.
    fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }
}

/// Verifies first launch creates log directory before file.
#[test]
fn first_launch_creates_log_directory_before_file() {
    let host = Arc::new(TestFileSystemHost::default());
    let files: Arc<dyn FileSystemHost> = host.clone();
    ensure_log_file(&files, "/runtime/logs/operit.log").unwrap();
    let storage = host.storage.lock().unwrap();
    assert!(storage.directories.contains("/runtime/logs"));
    assert_eq!(storage.files.get("/runtime/logs/operit.log").unwrap(), "");
}

/// Verifies existing log contents are preserved.
#[test]
fn existing_log_contents_are_preserved() {
    let host = Arc::new(TestFileSystemHost::default());
    host.storage.lock().unwrap().files.insert(
        "/runtime/logs/operit.log".into(),
        "previous session logs".into(),
    );
    let files: Arc<dyn FileSystemHost> = host.clone();
    ensure_log_file(&files, "/runtime/logs/operit.log").unwrap();
    assert_eq!(
        host.storage.lock().unwrap().files["/runtime/logs/operit.log"],
        "previous session logs"
    );
}

/// Verifies directory at log path is reported.
#[test]
fn directory_at_log_path_is_reported() {
    let host = Arc::new(TestFileSystemHost::default());
    host.storage
        .lock()
        .unwrap()
        .directories
        .insert("/runtime/logs/operit.log".into());
    let files: Arc<dyn FileSystemHost> = host;
    let error = ensure_log_file(&files, "/runtime/logs/operit.log").unwrap_err();
    assert_eq!(
        error,
        "Log file path is a directory: '/runtime/logs/operit.log'"
    );
}

/// Verifies host failures include operation path and cause.
#[test]
fn host_failures_include_operation_path_and_cause() {
    let cases = [
        (
            TestFileSystemHost {
                fail_inspect: true,
                ..Default::default()
            },
            "Cannot inspect log file '/runtime/logs/operit.log': Permission denied inspecting file",
        ),
        (
            TestFileSystemHost {
                fail_directory: true,
                ..Default::default()
            },
            "Cannot create log directory '/runtime/logs': Permission denied creating directory",
        ),
        (
            TestFileSystemHost {
                fail_write: Some("/runtime/logs/operit.log".into()),
                ..Default::default()
            },
            "Cannot create log file '/runtime/logs/operit.log': Permission denied writing file",
        ),
    ];
    for (host, expected) in cases {
        let files: Arc<dyn FileSystemHost> = Arc::new(host);
        assert_eq!(
            ensure_log_file(&files, "/runtime/logs/operit.log").unwrap_err(),
            expected
        );
    }
}

/// Verifies failed configuration keeps memory logs and can recover.
#[test]
fn failed_configuration_keeps_memory_logs_and_can_recover() {
    let _global = global_log_guard();
    const LOG: &str = "/runtime/logs/operit.log";
    const PACKAGE_LOG: &str = "/runtime/logs/toolpkg.log";
    let first = Arc::new(TestFileSystemHost::default());
    AppLogger::configure_log_files(first.clone(), LOG.into(), PACKAGE_LOG.into()).unwrap();
    AppLogger::i("BootstrapTest", "before failure");
    let previous_log = first.storage.lock().unwrap().files[LOG].clone();

    // Fail on the second file: a partially prepared pair must not stay bound.
    let failing = Arc::new(TestFileSystemHost {
        fail_write: Some(PACKAGE_LOG.into()),
        ..Default::default()
    });
    let error =
        AppLogger::configure_log_files(failing, LOG.into(), PACKAGE_LOG.into()).unwrap_err();
    assert!(error.contains(PACKAGE_LOG));
    assert!(!AppLogger::enable_file_logging());
    assert!(AppLogger::get_log_file().is_none());
    assert!(AppLogger::get_package_log_file().is_none());
    AppLogger::w("BootstrapTest", "continuing without file logs");
    assert!(AppLogger::entries()
        .iter()
        .any(|entry| entry.message == "continuing without file logs"));
    assert_eq!(first.storage.lock().unwrap().files[LOG], previous_log);

    let recovered = Arc::new(TestFileSystemHost::default());
    AppLogger::configure_log_files(recovered.clone(), LOG.into(), PACKAGE_LOG.into()).unwrap();
    assert!(AppLogger::enable_file_logging());
    AppLogger::i("ToolPkg", "file logging recovered");
    let storage = recovered.storage.lock().unwrap();
    assert!(storage.files[LOG].contains("file logging recovered"));
    assert!(storage.files[PACKAGE_LOG].contains("file logging recovered"));
}

/// Coalesces nested synchronous scopes while preserving formatted records and explicit host ownership.
#[test]
fn file_write_batches_preserve_records_and_flush_once_per_host() {
    let first = Arc::new(TestFileSystemHost::default());
    let second = Arc::new(TestFileSystemHost::default());
    let first_host: Arc<dyn FileSystemHost> = first.clone();
    let second_host: Arc<dyn FileSystemHost> = second.clone();
    let path = "/runtime/logs/operit.log";
    ensure_log_file(&first_host, path).unwrap();
    ensure_log_file(&second_host, path).unwrap();
    {
        let _outer = AppLogger::batch_file_writes();
        append_line(&first_host, path, "12:00:00.001 first\n").unwrap();
        {
            let _inner = AppLogger::batch_file_writes();
            append_line(&first_host, path, "12:00:00.002 second\n").unwrap();
            append_line(&second_host, path, "12:00:00.003 other-host\n").unwrap();
        }
        assert_eq!(first.storage.lock().unwrap().files[path], "");
    }
    assert_eq!(
        first.storage.lock().unwrap().files[path],
        "12:00:00.001 first\n12:00:00.002 second\n"
    );
    assert_eq!(
        second.storage.lock().unwrap().files[path],
        "12:00:00.003 other-host\n"
    );
    assert_eq!(
        first
            .append_count
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        second
            .append_count
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    append_line(&first_host, path, "12:00:00.004 outside\n").unwrap();
    assert_eq!(
        first
            .append_count
            .load(std::sync::atomic::Ordering::Relaxed),
        2
    );
}

/// Flushes records before a failed synchronous execution leaves its thread.
#[test]
fn file_write_batches_flush_when_execution_unwinds() {
    let host = Arc::new(TestFileSystemHost::default());
    let files: Arc<dyn FileSystemHost> = host.clone();
    let path = "/runtime/logs/operit.log";
    ensure_log_file(&files, path).unwrap();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _batch = AppLogger::batch_file_writes();
        append_line(&files, path, "execution failed\n").unwrap();
        panic!("fixture execution failure");
    }));
    assert!(failure.is_err());
    assert_eq!(
        host.storage.lock().unwrap().files[path],
        "execution failed\n"
    );
    assert_eq!(
        host.append_count.load(std::sync::atomic::Ordering::Relaxed),
        1
    );
}

/// Flushes each poll before yielding and keeps unrelated work outside the suspended future's batch.
#[test]
fn future_file_write_batches_flush_before_pending() {
    use std::future::{poll_fn, Future};
    use std::task::{Context, Poll, Waker};
    let host = Arc::new(TestFileSystemHost::default());
    let files: Arc<dyn FileSystemHost> = host.clone();
    let path = "/runtime/logs/operit.log";
    ensure_log_file(&files, path).unwrap();
    let mut polls = 0;
    let mut future = Box::pin(AppLogger::batch_future_file_writes(poll_fn(|_| {
        append_line(&files, path, "first record\n").unwrap();
        append_line(&files, path, "second record\n").unwrap();
        polls += 1;
        if polls == 1 {
            Poll::Pending
        } else {
            Poll::Ready(42)
        }
    })));
    let _: &dyn Send = &future;
    let mut context = Context::from_waker(Waker::noop());
    assert!(future.as_mut().poll(&mut context).is_pending());
    assert_eq!(
        host.append_count.load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        host.storage.lock().unwrap().files[path],
        "first record\nsecond record\n"
    );
    append_line(&files, path, "unrelated task\n").unwrap();
    assert_eq!(
        host.append_count.load(std::sync::atomic::Ordering::Relaxed),
        2
    );
    assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(42));
    assert_eq!(
        host.append_count.load(std::sync::atomic::Ordering::Relaxed),
        3
    );
}

/// Builds one oversized log body with line-length-tagged records.
fn oversize_log_body(records: usize, line_bytes: usize) -> String {
    let filler = "x".repeat(line_bytes);
    (0..records)
        .map(|index| format!("record {index} {filler}\n"))
        .collect()
}

/// Evicts the oldest records of an oversized log file and keeps the newest.
#[test]
fn oversize_log_file_evicts_oldest_records() {
    let host = Arc::new(TestFileSystemHost::default());
    let files: Arc<dyn FileSystemHost> = host.clone();
    let path = "/runtime/logs/operit.log";
    ensure_log_file(&files, path).unwrap();
    let body = oversize_log_body(640, 8 * 1024);
    files.writeFile(path, &body, true).unwrap();
    assert!(host.storage.lock().unwrap().files[path].len() as i64 > LOG_FILE_MAX_BYTES);

    evict_log_file_oldest(&files, path).unwrap();
    let trimmed = host.storage.lock().unwrap().files[path].clone();
    assert!(trimmed.contains("oldest records evicted"));
    assert!(
        trimmed.contains("record 639 "),
        "newest records must survive"
    );
    assert!(
        !trimmed.contains("record 0 "),
        "oldest records must be evicted"
    );
    assert!(
        trimmed.len() > LOG_FILE_KEEP_BYTES as usize / 2,
        "roughly half the cap is kept"
    );
    assert!(trimmed.lines().next().unwrap().contains("AppLogger"));

    let second_pass = trimmed.clone();
    evict_log_file_oldest(&files, path).unwrap();
    assert_eq!(
        host.storage.lock().unwrap().files[path],
        second_pass,
        "must be a no-op under the cap"
    );
}

/// Trims a legacy oversized log file when the logger binds its path.
#[test]
fn binding_trims_oversized_legacy_log_file() {
    let host = Arc::new(TestFileSystemHost::default());
    let path = "/runtime/logs/operit.log";
    {
        let mut storage = host.storage.lock().unwrap();
        storage.directories.insert("/runtime/logs".into());
        storage
            .files
            .insert(path.into(), oversize_log_body(640, 8 * 1024));
    }
    let files: Arc<dyn FileSystemHost> = host.clone();
    ensure_log_file(&files, path).unwrap();
    let trimmed = host.storage.lock().unwrap().files[path].clone();
    assert!(trimmed.len() as i64 <= LOG_FILE_MAX_BYTES);
    assert!(trimmed.contains("oldest records evicted"));
}

/// Keeps whole records only, including across multi-byte character cuts.
#[test]
fn newest_log_records_keeps_whole_lines_only() {
    let content = "first\nsecond\nthird\n";
    assert_eq!(newest_log_records(content, 50).as_deref(), Some(content));
    assert_eq!(
        newest_log_records(content, 8).as_deref(),
        Some("third\n"),
        "a cut inside a record drops that partial record"
    );
    assert_eq!(
        newest_log_records("é\n中文记录\nlast\n", 6).as_deref(),
        Some("last\n"),
        "the cut advances past a split multi-byte character"
    );
    assert_eq!(newest_log_records("one-single-record", 4), None);
}

/// Evicts the oldest in-memory records once the history exceeds its cap.
#[test]
fn memory_history_is_capped_and_keeps_newest_entries() {
    let mut entries: Vec<LogEntry> = Vec::new();
    for index in 0..(MAX_MEMORY_ENTRIES + 1024) {
        push_entry_capped(
            &mut entries,
            LogEntry {
                priority: INFO,
                verbose_level: None,
                tag: "RetentionTest".into(),
                message: format!("entry {index}"),
                throwable: None,
                timestamp_ms: index as u128,
            },
        );
    }
    // One overflow pass drains down to KEEP; the remainder never crosses the cap again.
    let total = MAX_MEMORY_ENTRIES + 1024;
    assert_eq!(
        entries.len(),
        KEEP_MEMORY_ENTRIES + (total - MAX_MEMORY_ENTRIES - 1)
    );
    assert!(entries.len() <= MAX_MEMORY_ENTRIES);
    assert_eq!(
        entries[0].message,
        format!("entry {}", MAX_MEMORY_ENTRIES + 1 - KEEP_MEMORY_ENTRIES)
    );
    assert_eq!(
        entries.last().unwrap().message,
        format!("entry {}", total - 1)
    );
}

/// Verifies file writes run while the state lock is held.
#[test]
fn file_writes_run_while_the_state_lock_is_held() {
    let _global = global_log_guard();
    const LOG: &str = "/runtime/logs/operit.log";
    const PACKAGE_LOG: &str = "/runtime/logs/toolpkg.log";

    let gate = Arc::new(WriteGate::new());
    let host = Arc::new(TestFileSystemHost {
        write_gate: Some(gate.clone()),
        ..Default::default()
    });
    AppLogger::configure_log_files(host, LOG.into(), PACKAGE_LOG.into()).unwrap();

    let writer = std::thread::spawn(|| AppLogger::i("GateTest", "line written under lock"));
    if !gate.wait_entered(Duration::from_secs(5)) {
        gate.release(); // never leave a writer parked on a failing run
        panic!("writer never reached the host file write");
    }

    let other = std::thread::spawn(|| AppLogger::set_enable_file_logging(true));
    // The writer is parked inside the host write. If file operations happen
    // while the state lock is held, `other` cannot finish until release.
    std::thread::sleep(Duration::from_millis(150));
    let finished_while_parked = other.is_finished();
    gate.release();
    let _ = writer.join();
    let _ = other.join();
    assert!(
        !finished_while_parked,
        "a second thread took the state lock while a file write was in flight"
    );
}

/// Verifies concurrent appends never lose lines across rotation.
#[test]
fn concurrent_appends_never_lose_lines_across_rotation() {
    const LOG: &str = "/runtime/logs/operit.log";
    const CAP: i64 = 2048;
    const THREADS: usize = 4;
    const LINES_PER_THREAD: usize = 30;
    const ROUNDS: usize = 60;
    const DIRECTORY: &str = "/runtime/logs";

    for round in 0..ROUNDS {
        let prefill = "x".repeat(CAP as usize + 64);
        let host = Arc::new(TestFileSystemHost::default());
        {
            let mut storage = host.storage.lock().unwrap();
            storage.directories.insert(DIRECTORY.into());
            storage.files.insert(LOG.into(), prefill.clone());
        }

        let barrier = Arc::new(Barrier::new(THREADS));
        let writers: Vec<_> = (0..THREADS)
            .map(|thread| {
                let host = host.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let files: Arc<dyn FileSystemHost> = host.clone();
                    barrier.wait();
                    for line in 0..LINES_PER_THREAD {
                        append_line_with_rotation_cap(
                            &files,
                            LOG,
                            &format!("T{thread}L{line:03}\n"),
                            CAP,
                        )
                        .expect("an append under a healthy host must succeed");
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }

        let storage = host.storage.lock().unwrap();
        let archive = storage
            .files
            .get(&format!("{LOG}.1"))
            .cloned()
            .unwrap_or_default();
        let current = storage.files.get(LOG).cloned().unwrap_or_default();
        assert_eq!(
            archive, prefill,
            "round {round}: the single archive must hold exactly the pre-rotation file"
        );
        for thread in 0..THREADS {
            for line in 0..LINES_PER_THREAD {
                let expected = format!("T{thread}L{line:03}\n");
                assert_eq!(
                    current.matches(&expected).count(),
                    1,
                    "round {round}: line {expected:?} was lost or duplicated"
                );
            }
        }
        assert_eq!(
            current.matches('\n').count(),
            THREADS * LINES_PER_THREAD,
            "round {round}: unexpected extra or missing lines"
        );
    }
}

/// Verifies rotation failure is propagated and writes no line.
#[test]
fn rotation_failure_is_propagated_and_writes_no_line() {
    const LOG: &str = "/runtime/logs/operit.log";
    const CAP: i64 = 2048;
    let prefill = "x".repeat(CAP as usize + 64);
    let host = Arc::new(TestFileSystemHost {
        fail_move: Some(LOG.into()),
        ..Default::default()
    });
    {
        let mut storage = host.storage.lock().unwrap();
        storage.directories.insert("/runtime/logs".into());
        storage.files.insert(LOG.into(), prefill.clone());
    }

    let files: Arc<dyn FileSystemHost> = host.clone();
    let outcome = append_line_with_rotation_cap(&files, LOG, "dropped\n", CAP);
    assert!(
        outcome.is_err(),
        "a failed rotation must surface as an explicit Err, not as a logged-and-continue"
    );

    let storage = host.storage.lock().unwrap();
    assert_eq!(
        storage.files[LOG], prefill,
        "after a failed rotation no line may be appended to the oversized file"
    );
    assert!(
        !storage.files.contains_key(&format!("{LOG}.1")),
        "the failed move must not produce an archive"
    );
}

/// Verifies append failure is propagated as err.
#[test]
fn append_failure_is_propagated_as_err() {
    const LOG: &str = "/runtime/logs/operit.log";
    const CAP: i64 = 8192;
    let host = Arc::new(TestFileSystemHost {
        fail_write: Some(LOG.into()),
        ..Default::default()
    });
    host.storage
        .lock()
        .unwrap()
        .directories
        .insert("/runtime/logs".into());

    let files: Arc<dyn FileSystemHost> = host.clone();
    let outcome = append_line_with_rotation_cap(&files, LOG, "never lands\n", CAP);
    assert!(outcome.is_err(), "a failed append must be Err");
    assert!(
        !host.storage.lock().unwrap().files.contains_key(LOG),
        "nothing may be written when the append fails"
    );
}

/// Verifies report log io error is infallible and does not panic.
#[test]
fn report_log_io_error_is_infallible_and_does_not_panic() {
    // The report path runs while the state lock is held; it must not panic
    // (closed stderr) and must not re-enter the logger (deadlock).
    report_log_io_error("stat", "/runtime/logs/operit.log", "simulated failure");
}

/// Verifies write entry surfaces file failures to the caller.
#[test]
fn write_entry_surfaces_file_failures_to_the_caller() {
    let _global = global_log_guard();
    const LOG: &str = "/runtime/logs/operit.log";
    const PACKAGE_LOG: &str = "/runtime/logs/toolpkg.log";

    // Oversized runtime log whose rotation move fails: the bound sink is
    // configured, but every write through it fails. write_entry uses the
    // production cap, so the file must exceed LOG_ROTATE_BYTES.
    let host = Arc::new(TestFileSystemHost {
        fail_move: Some(LOG.into()),
        ..Default::default()
    });
    AppLogger::configure_log_files(host.clone(), LOG.into(), PACKAGE_LOG.into()).unwrap();
    host.storage
        .lock()
        .unwrap()
        .files
        .insert(LOG.into(), "x".repeat(LOG_ROTATE_BYTES as usize + 64));

    let outcome = write_entry(ERROR, None, "FailureTest", "line with a broken sink", None);
    assert!(
        outcome.is_err(),
        "write_entry must propagate a file sink failure as a value"
    );
    assert_eq!(
        AppLogger::e("FailureTest", "second failure with a bound sink"),
        -1,
        "the public write API must expose the failure on its return value"
    );
}

/// Routes batch flushes through rotation while preserving one append per file.
#[test]
fn batch_flush_rotates_oversized_files_and_preserves_all_queued_records() {
    const LOG: &str = "/runtime/logs/operit.log";
    let host = Arc::new(TestFileSystemHost::default());
    let files: Arc<dyn FileSystemHost> = host.clone();
    ensure_log_file(&files, LOG).unwrap();
    let original = "x".repeat(LOG_ROTATE_BYTES as usize + 64);
    host.storage
        .lock()
        .unwrap()
        .files
        .insert(LOG.into(), original.clone());
    {
        let _batch = AppLogger::batch_file_writes();
        append_line(&files, LOG, "first queued record\n").unwrap();
        append_line(&files, LOG, "second queued record\n").unwrap();
        assert_eq!(host.storage.lock().unwrap().files[LOG], original);
    }
    let storage = host.storage.lock().unwrap();
    assert_eq!(storage.files[&format!("{LOG}.1")], original);
    assert_eq!(
        storage.files[LOG],
        "first queued record\nsecond queued record\n"
    );
    assert_eq!(
        host.append_count.load(std::sync::atomic::Ordering::Relaxed),
        1
    );
}

/// Makes diagnostic reads wait for a rotation's missing-path interval to finish.
#[test]
fn diagnostic_reads_are_serialized_with_rotation() {
    let _global = global_log_guard();
    const LOG: &str = "/runtime/logs/operit.log";
    const PACKAGE_LOG: &str = "/runtime/logs/toolpkg.log";
    let gate = Arc::new(WriteGate::new());
    let host = Arc::new(TestFileSystemHost {
        move_gate: Some(gate.clone()),
        ..Default::default()
    });
    AppLogger::configure_log_files(host.clone(), LOG.into(), PACKAGE_LOG.into()).unwrap();
    host.storage
        .lock()
        .unwrap()
        .files
        .insert(LOG.into(), "x".repeat(128));
    let files: Arc<dyn FileSystemHost> = host;
    let writer = std::thread::spawn(move || {
        append_line_with_rotation_cap(&files, LOG, "after rotation\n", 64)
    });
    if !gate.wait_entered(Duration::from_secs(5)) {
        gate.release();
        writer.join().unwrap().unwrap();
        panic!("writer never entered rotation");
    }
    let reader = std::thread::spawn(AppLogger::text);
    std::thread::sleep(Duration::from_millis(150));
    let read_finished_during_rotation = reader.is_finished();
    gate.release();
    writer.join().unwrap().unwrap();
    let text = reader.join().unwrap().unwrap();
    assert!(
        !read_finished_during_rotation,
        "export read an incomplete rotation"
    );
    assert_eq!(text, "after rotation\n");
}

/// Deletes the previous archive generation during the next rotation.
#[test]
fn repeated_rotation_keeps_only_the_latest_archive_generation() {
    const LOG: &str = "/runtime/logs/operit.log";
    let host = Arc::new(TestFileSystemHost::default());
    let files: Arc<dyn FileSystemHost> = host.clone();
    ensure_log_file(&files, LOG).unwrap();
    for content in ["first generation\n", "second generation\n"] {
        host.storage
            .lock()
            .unwrap()
            .files
            .insert(LOG.into(), content.into());
        append_line_with_rotation_cap(&files, LOG, "new current record\n", 4).unwrap();
    }
    let storage = host.storage.lock().unwrap();
    assert_eq!(storage.files[&format!("{LOG}.1")], "second generation\n");
    assert_eq!(storage.files[LOG], "new current record\n");
    assert_eq!(storage.files.len(), 2);
}

/// Surfaces errors while preparing an oversized legacy log for diagnostic export.
#[test]
fn binding_reports_legacy_log_trim_write_failures() {
    const LOG: &str = "/runtime/logs/operit.log";
    let host = Arc::new(TestFileSystemHost {
        fail_write: Some(LOG.into()),
        ..Default::default()
    });
    host.storage
        .lock()
        .unwrap()
        .files
        .insert(LOG.into(), oversize_log_body(640, 8 * 1024));
    let files: Arc<dyn FileSystemHost> = host;
    assert_eq!(
        ensure_log_file(&files, LOG).unwrap_err(),
        "Permission denied writing file"
    );
}
