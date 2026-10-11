use super::*;
use operit_host_api::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Barrier, Condvar};
use std::time::{Duration, Instant};

/// Serializes tests that bind the process-global AppLogger state.
static GLOBAL_LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

fn global_log_guard() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_LOG_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Gates appends so a test can park one writer inside the host file write.
struct WriteGate {
    state: Mutex<(bool, bool)>, // (entered, proceed)
    cv: Condvar,
}

impl WriteGate {
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
    fail_move: Option<String>,
    write_gate: Option<Arc<WriteGate>>,
}

impl FileSystemHost for TestFileSystemHost {
    fn envLabel(&self) -> &str {
        "test"
    }

    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::linux()
    }

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

    fn validatePath(&self, _path: &str, _paramName: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    fn listFiles(&self, _path: &str) -> HostResult<Vec<FileEntry>> {
        unreachable!("unrelated file-system operation")
    }

    fn readFile(&self, _path: &str) -> HostResult<String> {
        unreachable!("unrelated file-system operation")
    }

    fn readFileWithLimit(&self, _path: &str, _maxBytes: usize) -> HostResult<String> {
        unreachable!("unrelated file-system operation")
    }

    fn readFileBytes(&self, _path: &str) -> HostResult<Vec<u8>> {
        unreachable!("unrelated file-system operation")
    }

    fn writeFileBytes(&self, _path: &str, _content: &[u8]) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    fn deleteFile(&self, path: &str, _recursive: bool) -> HostResult<()> {
        let mut storage = self.storage.lock().unwrap();
        if storage.files.remove(path).is_some() {
            Ok(())
        } else {
            Err(HostError::new(format!("No such file: {path}")))
        }
    }

    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        if self.fail_move.as_deref() == Some(source) {
            return Err(HostError::new("Permission denied moving file"));
        }
        let mut storage = self.storage.lock().unwrap();
        match storage.files.remove(source) {
            Some(content) => {
                storage.files.insert(destination.into(), content);
                Ok(())
            }
            None => Err(HostError::new(format!("No such file: {source}"))),
        }
    }

    fn copyFile(&self, _source: &str, _destination: &str, _recursive: bool) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
        unreachable!("unrelated file-system operation")
    }

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

    fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        unreachable!("unrelated file-system operation")
    }

    fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    fn openFile(&self, _path: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }

    fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
        unreachable!("unrelated file-system operation")
    }
}

#[test]
fn first_launch_creates_log_directory_before_file() {
    let host = Arc::new(TestFileSystemHost::default());
    let files: Arc<dyn FileSystemHost> = host.clone();
    ensure_log_file(&files, "/runtime/logs/operit.log").unwrap();
    let storage = host.storage.lock().unwrap();
    assert!(storage.directories.contains("/runtime/logs"));
    assert_eq!(storage.files.get("/runtime/logs/operit.log").unwrap(), "");
}

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
                        );
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

#[test]
fn rotation_failure_still_appends_and_keeps_the_failure_visible() {
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
    append_line_with_rotation_cap(&files, LOG, "survivor\n", CAP);

    let storage = host.storage.lock().unwrap();
    assert_eq!(
        storage.files[LOG],
        format!("{prefill}survivor\n"),
        "a failed rotation must not drop the appended line"
    );
    assert!(
        !storage.files.contains_key(&format!("{LOG}.1")),
        "the failed move must not produce an archive"
    );
}
