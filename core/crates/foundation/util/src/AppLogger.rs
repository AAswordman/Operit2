#[cfg(not(target_os = "espidf"))]
use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, Once, OnceLock};

#[cfg(not(target_os = "espidf"))]
use chrono::{DateTime, Local, Utc};
use operit_host_api::FileSystemHost;
use serde::{Deserialize, Serialize};

/// Android-compatible verbose log priority.
pub const VERBOSE: i32 = 2;
/// Android-compatible debug log priority.
pub const DEBUG: i32 = 3;
/// Android-compatible info log priority.
pub const INFO: i32 = 4;
/// Android-compatible warning log priority.
pub const WARN: i32 = 5;
/// Android-compatible error log priority.
pub const ERROR: i32 = 6;
/// Android-compatible assert log priority.
pub const ASSERT: i32 = 7;

/// Process and configuration lifecycle diagnostics (V1).
pub const VERBOSE_LEVEL_1: i32 = 1;
/// User-operation stages and aggregate timings (V2).
pub const VERBOSE_LEVEL_2: i32 = 2;
/// Service, provider, plugin, and tool invocation boundaries (V3).
pub const VERBOSE_LEVEL_3: i32 = 3;
/// Component, session, and resource lifecycle diagnostics (V4).
pub const VERBOSE_LEVEL_4: i32 = 4;
/// Routing, scheduling, persistence, and state propagation internals (V5).
pub const VERBOSE_LEVEL_5: i32 = 5;
/// Per-item, per-chunk, repeated-loop, and payload diagnostics (V6).
pub const VERBOSE_LEVEL_6: i32 = 6;

const TOOLPKG_LOG_TAG: &str = "ToolPkg";

/// Size cap for one log file; crossing it evicts the oldest records.
const LOG_FILE_MAX_BYTES: i64 = 4 * 1024 * 1024;
/// Newest bytes kept after one log file eviction pass.
const LOG_FILE_KEEP_BYTES: i64 = 2 * 1024 * 1024;
/// Cap for the in-memory entry history.
const MAX_MEMORY_ENTRIES: usize = 4096;
/// Entries kept after one in-memory eviction pass.
const KEEP_MEMORY_ENTRIES: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// One in-memory and file-backed application log entry.
pub struct LogEntry {
    pub priority: i32,
    pub verbose_level: Option<i32>,
    pub tag: String,
    pub message: String,
    pub throwable: Option<String>,
    pub timestamp_ms: u128,
}

struct LoggerState {
    enable_file_logging: bool,
    enable_console_logging: bool,
    console_min_level: i32,
    console_max_verbose_level: i32,
    file_system_host: Option<Arc<dyn FileSystemHost>>,
    log_file: Option<String>,
    package_log_file: Option<String>,
    entries: Vec<LogEntry>,
    retain_entries: bool,
}

/// Retains formatted file records only for the current synchronous execution step.
struct BatchedLogFileWrite {
    host: Arc<dyn FileSystemHost>,
    path: String,
    content: String,
}

#[derive(Default)]
struct LogFileWriteBatchState {
    depth: usize,
    writes: Vec<BatchedLogFileWrite>,
}

thread_local! {
    static FILE_WRITE_BATCH: RefCell<LogFileWriteBatchState> = RefCell::new(LogFileWriteBatchState::default());
}

/// Flushes each file once when the owning thread leaves its outermost synchronous batch.
pub struct LogFileWriteBatch {
    owner_thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl Drop for LogFileWriteBatch {
    /// Flushes after releasing the thread-local borrow, including during error unwinding.
    fn drop(&mut self) {
        let writes = FILE_WRITE_BATCH.with(|batch| {
            let mut batch = batch.borrow_mut();
            batch.depth -= 1;
            if batch.depth == 0 {
                std::mem::take(&mut batch.writes)
            } else {
                Vec::new()
            }
        });
        if writes.is_empty() {
            return;
        }
        let _state_guard = state().lock().expect("AppLogger mutex poisoned");
        for write in writes {
            let _ = append_line_with_rotation_cap(
                &write.host,
                &write.path,
                &write.content,
                LOG_ROTATE_BYTES,
            );
        }
    }
}

static STATE: OnceLock<Mutex<LoggerState>> = OnceLock::new();
static HOST_LOG_SINK_INIT: Once = Once::new();
/// Serializes file reads, rotation, preparation, resets, and appends.
static FILE_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Returns the process-wide logger state and installs the host log sink.
fn state() -> &'static Mutex<LoggerState> {
    install_host_log_sink_once();
    STATE.get_or_init(|| {
        Mutex::new(LoggerState {
            enable_file_logging: true,
            enable_console_logging: true,
            console_min_level: INFO,
            console_max_verbose_level: VERBOSE_LEVEL_6,
            file_system_host: None,
            log_file: None,
            package_log_file: None,
            entries: Vec::new(),
            retain_entries: true,
        })
    })
}

/// Connects host diagnostics to AppLogger once per process.
fn install_host_log_sink_once() {
    HOST_LOG_SINK_INIT.call_once(|| {
        operit_host_api::setHostLogSink(std::sync::Arc::new(|tag, message| {
            AppLogger::e(tag, message);
        }));
    });
}

/// Process-wide application logger used by host and runtime code.
pub struct AppLogger;

impl AppLogger {
    /// Coalesces file writes for a synchronous scope while preserving individual records and timestamps.
    pub fn batch_file_writes() -> LogFileWriteBatch {
        FILE_WRITE_BATCH.with(|batch| batch.borrow_mut().depth += 1);
        LogFileWriteBatch {
            owner_thread: std::marker::PhantomData,
        }
    }

    /// Batches only each synchronous poll and flushes before a future yields or changes threads.
    pub async fn batch_future_file_writes<F: std::future::Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        std::future::poll_fn(|context| {
            let _batch = Self::batch_file_writes();
            future.as_mut().poll(context)
        })
        .await
    }

    /// Controls in-memory history independently of console/file sinks.
    /// Constrained apps may disable retention without changing other hosts.
    pub fn set_retain_entries(enabled: bool) {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.retain_entries = enabled;
        if !enabled {
            guard.entries = Vec::new();
        }
    }

    /// Enables or disables file logging.
    pub fn set_enable_file_logging(enabled: bool) {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.enable_file_logging = enabled;
    }

    /// Returns whether file logging is enabled.
    pub fn enable_file_logging() -> bool {
        state()
            .lock()
            .expect("AppLogger mutex poisoned")
            .enable_file_logging
    }

    /// Enables or disables console logging.
    pub fn set_enable_console_logging(enabled: bool) {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.enable_console_logging = enabled;
    }

    /// Returns whether console logging is enabled.
    pub fn enable_console_logging() -> bool {
        state()
            .lock()
            .expect("AppLogger mutex poisoned")
            .enable_console_logging
    }

    /// Sets the minimum severity emitted to the live console.
    pub fn set_console_min_level(level: i32) -> Result<(), String> {
        if level < VERBOSE || level > ASSERT {
            return Err(format!(
                "console log level must be between {VERBOSE} and {ASSERT}"
            ));
        }
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.console_min_level = level;
        Ok(())
    }

    /// Returns the minimum severity emitted to the live console.
    pub fn console_min_level() -> i32 {
        state()
            .lock()
            .expect("AppLogger mutex poisoned")
            .console_min_level
    }

    /// Sets the maximum Verbose detail level emitted to the live console.
    pub fn set_console_max_verbose_level(level: i32) -> Result<(), String> {
        if !(VERBOSE_LEVEL_1..=VERBOSE_LEVEL_6).contains(&level) {
            return Err(format!(
                "Verbose detail level must be between {VERBOSE_LEVEL_1} and {VERBOSE_LEVEL_6}"
            ));
        }
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.console_max_verbose_level = level;
        Ok(())
    }

    /// Returns the maximum Verbose detail level emitted to the live console.
    pub fn console_max_verbose_level() -> i32 {
        state()
            .lock()
            .expect("AppLogger mutex poisoned")
            .console_max_verbose_level
    }

    /// Configures runtime and ToolPkg logs through the supplied file-system host.
    /// Returns preparation failures and disables unprepared file sinks.
    pub fn configure_log_files(
        file_system_host: Arc<dyn FileSystemHost>,
        log_file: String,
        package_log_file: String,
    ) -> Result<(), String> {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        if let Err(error) = ensure_log_file(&file_system_host, &log_file)
            .and_then(|()| ensure_log_file(&file_system_host, &package_log_file))
        {
            guard.file_system_host = None;
            guard.log_file = None;
            guard.package_log_file = None;
            guard.enable_file_logging = false;
            return Err(error);
        }
        guard.file_system_host = Some(file_system_host);
        guard.log_file = Some(log_file);
        guard.package_log_file = Some(package_log_file);
        guard.enable_file_logging = true;
        Ok(())
    }

    /// Returns the bound runtime log file path.
    pub fn get_log_file() -> Option<String> {
        state()
            .lock()
            .expect("AppLogger mutex poisoned")
            .log_file
            .clone()
    }

    /// Returns the bound ToolPkg log file path.
    pub fn get_package_log_file() -> Option<String> {
        state()
            .lock()
            .expect("AppLogger mutex poisoned")
            .package_log_file
            .clone()
    }

    /// Returns the runtime log file path as display text.
    pub fn get_log_file_path() -> Result<String, String> {
        Self::get_log_file().ok_or_else(|| "AppLogger log file is not bound".to_string())
    }

    /// Returns the ToolPkg log file path as display text.
    pub fn get_package_log_file_path() -> Result<String, String> {
        Self::get_package_log_file()
            .ok_or_else(|| "AppLogger package log file is not bound".to_string())
    }

    /// Clears current log files and in-memory log entries.
    pub fn reset_log_file() {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        let _file_guard = FILE_WRITE_LOCK
            .lock()
            .expect("AppLogger file mutex poisoned");
        if let Some(file_system_host) = &guard.file_system_host {
            if let Some(path) = &guard.log_file {
                let _ = file_system_host.writeFile(path, "", false);
            }
            if let Some(path) = &guard.package_log_file {
                let _ = file_system_host.writeFile(path, "", false);
            }
        }
        guard.entries.clear();
    }

    /// Returns a snapshot of in-memory log entries.
    pub fn entries() -> Vec<LogEntry> {
        state()
            .lock()
            .expect("AppLogger mutex poisoned")
            .entries
            .clone()
    }

    /// Returns in-memory log entries as JSON.
    pub fn entries_json() -> serde_json::Value {
        serde_json::to_value(Self::entries()).expect("LogEntry serialization must succeed")
    }

    /// Reads the runtime log file as text.
    pub fn text() -> Result<String, String> {
        let (file_system_host, path) = log_file_access(false)?;
        let _file_guard = FILE_WRITE_LOCK
            .lock()
            .map_err(|_| "AppLogger file mutex poisoned".to_string())?;
        file_system_host
            .readFile(&path)
            .map_err(|error| error.message)
    }

    /// Reads the ToolPkg log file as text.
    pub fn package_text() -> Result<String, String> {
        let (file_system_host, path) = log_file_access(true)?;
        let _file_guard = FILE_WRITE_LOCK
            .lock()
            .map_err(|_| "AppLogger file mutex poisoned".to_string())?;
        file_system_host
            .readFile(&path)
            .map_err(|error| error.message)
    }

    /// Writes process or configuration lifecycle diagnostics at Verbose level one.
    pub fn v(tag: &str, msg: &str) -> i32 {
        Self::v_with_level(tag, msg, VERBOSE_LEVEL_1)
    }

    /// Writes diagnostics at an explicit depth from lifecycle (V1) through per-item detail (V6).
    pub fn v_with_level(tag: &str, msg: &str, verbose_level: i32) -> i32 {
        assert!((VERBOSE_LEVEL_1..=VERBOSE_LEVEL_6).contains(&verbose_level));
        match write_entry(VERBOSE, Some(verbose_level), tag, msg, None) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    /// Writes routing, scheduling, persistence, or state propagation diagnostics at V5.
    pub fn trace(tag: &str, msg: &str) -> i32 {
        Self::v_with_level(tag, msg, VERBOSE_LEVEL_5)
    }

    /// Writes per-item, per-chunk, or repeated-loop diagnostics at V6.
    pub fn trace_detail(tag: &str, msg: &str) -> i32 {
        Self::v_with_level(tag, msg, VERBOSE_LEVEL_6)
    }

    /// Writes a debug log message.
    pub fn d(tag: &str, msg: &str) -> i32 {
        Self::println(DEBUG, tag, msg)
    }

    /// Writes an info log message.
    pub fn i(tag: &str, msg: &str) -> i32 {
        Self::println(INFO, tag, msg)
    }

    /// Writes a warning log message.
    pub fn w(tag: &str, msg: &str) -> i32 {
        Self::println(WARN, tag, msg)
    }

    /// Writes an error log message.
    pub fn e(tag: &str, msg: &str) -> i32 {
        Self::println(ERROR, tag, msg)
    }

    /// Writes an assert-level log message.
    pub fn wtf(tag: &str, msg: &str) -> i32 {
        Self::println(ASSERT, tag, msg)
    }

    /// Writes a log message with an explicit priority.
    pub fn println(priority: i32, tag: &str, msg: &str) -> i32 {
        let verbose_level = (priority == VERBOSE).then_some(VERBOSE_LEVEL_1);
        match write_entry(priority, verbose_level, tag, msg, None) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    /// Writes a log message together with an error chain.
    pub fn println_with_error(
        priority: i32,
        tag: &str,
        msg: &str,
        tr: &(dyn std::error::Error),
    ) -> i32 {
        let verbose_level = (priority == VERBOSE).then_some(VERBOSE_LEVEL_1);
        match write_entry(priority, verbose_level, tag, msg, Some(error_chain(tr))) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    /// Captures the current stack trace as display text.
    pub fn get_stack_trace_string(_tr: &(dyn std::error::Error)) -> String {
        #[cfg(not(target_os = "espidf"))]
        {
            format!("{:?}", Backtrace::capture())
        }
        #[cfg(target_os = "espidf")]
        {
            "Backtrace unavailable on ESP-IDF".to_string()
        }
    }

    /// Returns whether a severity is enabled by the console threshold.
    pub fn is_loggable(_tag: &str, level: i32) -> bool {
        level >= Self::console_min_level()
    }

    /// Returns whether a Verbose detail level is enabled by the console thresholds.
    pub fn is_verbose_loggable(verbose_level: i32) -> bool {
        VERBOSE >= Self::console_min_level()
            && (VERBOSE_LEVEL_1..=Self::console_max_verbose_level()).contains(&verbose_level)
    }
}

/// Stores one log entry and emits it to the configured sinks.
///
/// Returns `Err` when any configured file sink failed, so the failure reaches
/// the caller as a value instead of only as a log line.
fn write_entry(
    priority: i32,
    verbose_level: Option<i32>,
    tag: &str,
    msg: &str,
    throwable: Option<String>,
) -> Result<(), ()> {
    let timestamp_ms = operit_host_api::TimeUtils::currentTimeMillisU128();
    let entry = LogEntry {
        priority,
        verbose_level,
        tag: tag.to_string(),
        message: msg.to_string(),
        throwable,
        timestamp_ms,
    };

    let line = format_log_line(&entry, tag);
    let package_line = tag
        .eq_ignore_ascii_case(TOOLPKG_LOG_TAG)
        .then(|| format_package_log_line(&entry));

    let (enable_console_logging, console_min_level, console_max_verbose_level, write_result) = {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        if guard.retain_entries {
            push_entry_capped(&mut guard.entries, entry.clone());
        }

        // File operations run while the state lock is held: the config check,
        // the rotation check, and the append are one critical section, so a
        // concurrent reset/reconfigure cannot slip in between them.
        let mut write_result = Ok(());
        if guard.enable_file_logging {
            if let (Some(file_system_host), Some(path)) = (&guard.file_system_host, &guard.log_file)
            {
                write_result = append_line(file_system_host, path, &line);
            }
            if let (Some(file_system_host), Some(path)) =
                (&guard.file_system_host, &guard.package_log_file)
            {
                if let Some(package_line) = &package_line {
                    // Keep the first failure: the runtime log line that could
                    // not be written is the one worth reporting.
                    if write_result.is_ok() {
                        write_result = append_line(file_system_host, path, package_line);
                    }
                }
            }
        }

        (
            guard.enable_console_logging,
            guard.console_min_level,
            guard.console_max_verbose_level,
            write_result,
        )
    };

    if enable_console_logging
        && is_console_loggable(
            priority,
            verbose_level,
            console_min_level,
            console_max_verbose_level,
        )
    {
        if !operit_host_api::tryLogHostConsole(priority, tag, &line) {
            use std::io::Write as _;
            match priority {
                ERROR | ASSERT => {
                    let _ = std::io::stderr().write_all(line.as_bytes());
                }
                _ => {
                    let _ = std::io::stdout().write_all(line.as_bytes());
                }
            }
        }
    }
    write_result
}

/// Appends one formatted log line through the configured file-system host.
///
/// Callers must hold the state lock; inside, rotation and the append also run
/// while holding [`FILE_WRITE_LOCK`] (lock order: state → file-write, never the
/// reverse) so concurrent writers cannot interleave the rotate/append sequence
/// and drop lines. Returns `Err` if the rotation or the append failed.
fn append_line(
    file_system_host: &Arc<dyn FileSystemHost>,
    path: &str,
    line: &str,
) -> Result<(), ()> {
    let batched = FILE_WRITE_BATCH.with(|batch| {
        let mut batch = batch.borrow_mut();
        if batch.depth == 0 {
            return false;
        }
        match batch
            .writes
            .iter_mut()
            .find(|write| write.path == path && Arc::ptr_eq(&write.host, file_system_host))
        {
            Some(write) => write.content.push_str(line),
            None => batch.writes.push(BatchedLogFileWrite {
                host: file_system_host.clone(),
                path: path.to_string(),
                content: line.to_string(),
            }),
        }
        true
    });
    if batched {
        return Ok(());
    }
    append_line_with_rotation_cap(file_system_host, path, line, LOG_ROTATE_BYTES)
}

/// [`append_line`] with an explicit rotation cap so tests can rotate cheaply.
fn append_line_with_rotation_cap(
    file_system_host: &Arc<dyn FileSystemHost>,
    path: &str,
    line: &str,
    rotate_bytes: i64,
) -> Result<(), ()> {
    let _guard = FILE_WRITE_LOCK
        .lock()
        .map_err(|_| report_log_io_error("acquire-file-write-lock", path, "lock poisoned"))?;
    rotate_if_oversized(file_system_host, path, rotate_bytes)?;
    file_system_host
        .writeFile(path, line, true)
        .map_err(|error| report_log_io_error("append", path, &error.message))
}

/// Size cap before a log file rotates (bytes).
const LOG_ROTATE_BYTES: i64 = LOG_FILE_MAX_BYTES;

/// Rotates one log file once it exceeds the size cap, keeping a single `.1` archive.
/// Returns an error before appending when any rotation operation fails.
fn rotate_if_oversized(
    file_system_host: &Arc<dyn FileSystemHost>,
    path: &str,
    rotate_bytes: i64,
) -> Result<(), ()> {
    let metadata = file_system_host
        .fileInfo(path)
        .map_err(|error| report_log_io_error("stat", path, &error.message))?;
    if metadata.size <= rotate_bytes {
        return Ok(());
    }
    let archive = format!("{path}.1");
    // Only touch a stale archive that actually exists — a missing archive is
    // the normal first rotation, not a failure worth reporting.
    match file_system_host.fileExists(&archive) {
        Ok(existence) if !existence.exists => {}
        Ok(_) => {
            file_system_host
                .deleteFile(&archive, false)
                .map_err(|error| report_log_io_error("delete-archive", &archive, &error.message))?;
        }
        Err(error) => {
            return Err(report_log_io_error(
                "inspect-archive",
                &archive,
                &error.message,
            ));
        }
    }
    file_system_host
        .moveFile(path, &archive)
        .map_err(|error| report_log_io_error("rotate-move", path, &error.message))?;
    file_system_host
        .writeFile(path, "", false)
        .map_err(|error| report_log_io_error("rotate-recreate", path, &error.message))
}

/// Reports file I/O failures to the host console and stderr without re-entering the logger.
fn report_log_io_error(action: &str, path: &str, message: &str) {
    use std::io::Write as _;
    let line = format!("AppLogger: failed to {action} '{path}': {message}\n");
    let _ = operit_host_api::tryLogHostConsole(ERROR, "AppLogger", &line);
    let _ = std::io::stderr().write_all(line.as_bytes());
}

/// Appends one entry, evicting the oldest records once the history exceeds its cap.
fn push_entry_capped(entries: &mut Vec<LogEntry>, entry: LogEntry) {
    entries.push(entry);
    let overflow = entries.len().saturating_sub(MAX_MEMORY_ENTRIES);
    if overflow > 0 {
        entries.drain(0..entries.len() - KEEP_MEMORY_ENTRIES);
    }
}

/// Drops the oldest records of one log file once it exceeds the size cap.
/// Returns preparation failures without re-entering the logger.
fn evict_log_file_oldest(
    file_system_host: &Arc<dyn FileSystemHost>,
    path: &str,
) -> Result<(), String> {
    let metadata = file_system_host
        .fileExists(path)
        .map_err(|error| error.message)?;
    if !metadata.exists || metadata.isDirectory || metadata.size <= LOG_FILE_MAX_BYTES {
        return Ok(());
    }
    let content = file_system_host
        .readFile(path)
        .map_err(|error| error.message)?;
    let Some(kept) = newest_log_records(&content, LOG_FILE_KEEP_BYTES as usize) else {
        return Err(format!(
            "Oversized log contains no complete records to retain: '{path}'"
        ));
    };
    let mut trimmed = format!(
        "{} I/AppLogger: log file exceeded the size limit; oldest records evicted\n",
        format_timestamp_ms(operit_host_api::TimeUtils::currentTimeMillisU128())
    );
    trimmed.push_str(&kept);
    file_system_host
        .writeFile(path, &trimmed, false)
        .map_err(|error| error.message)
}

/// Returns the newest records of one log text, keeping at most `keep_bytes`
/// bytes and starting at a record boundary; `None` when nothing can be kept.
fn newest_log_records(content: &str, keep_bytes: usize) -> Option<String> {
    if content.len() <= keep_bytes {
        return Some(content.to_string());
    }
    let mut cut = content.len() - keep_bytes;
    while cut < content.len() && !content.is_char_boundary(cut) {
        cut += 1;
    }
    match content[cut..].find('\n') {
        Some(offset) => Some(content[cut + offset + 1..].to_string()),
        None => None,
    }
}

/// Creates an empty log file through the host when the path is absent.
fn ensure_log_file(file_system_host: &Arc<dyn FileSystemHost>, path: &str) -> Result<(), String> {
    let _file_guard = FILE_WRITE_LOCK
        .lock()
        .map_err(|_| "AppLogger file mutex poisoned".to_string())?;
    let metadata = file_system_host
        .fileExists(path)
        .map_err(|error| format!("Cannot inspect log file '{path}': {}", error.message))?;
    if metadata.exists && metadata.isDirectory {
        return Err(format!("Log file path is a directory: '{path}'"));
    }
    if !metadata.exists {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let parent = parent.to_string_lossy();
            if !parent.is_empty() {
                file_system_host
                    .makeDirectory(&parent, true)
                    .map_err(|error| {
                        format!("Cannot create log directory '{parent}': {}", error.message)
                    })?;
            }
        }
        file_system_host
            .writeFile(path, "", false)
            .map_err(|error| format!("Cannot create log file '{path}': {}", error.message))?;
    } else if !metadata.isDirectory {
        // Trims oversized logs left behind by previous runs at bind time.
        evict_log_file_oldest(file_system_host, path)?;
    }
    Ok(())
}

/// Returns the host and path for one configured runtime or ToolPkg log file.
fn log_file_access(package_log: bool) -> Result<(Arc<dyn FileSystemHost>, String), String> {
    let guard = state().lock().expect("AppLogger mutex poisoned");
    let file_system_host = guard
        .file_system_host
        .clone()
        .ok_or_else(|| "AppLogger file-system host is not bound".to_string())?;
    let path = if package_log {
        guard.package_log_file.clone()
    } else {
        guard.log_file.clone()
    }
    .ok_or_else(|| "AppLogger log file is not bound".to_string())?;
    Ok((file_system_host, path))
}

/// Formats one runtime log record with its timestamp and error details.
fn format_log_line(entry: &LogEntry, tag: &str) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{} {}/{}: {}",
        format_timestamp_ms(entry.timestamp_ms),
        priority_label(entry.priority, entry.verbose_level),
        tag,
        entry.message
    );
    if let Some(throwable) = &entry.throwable {
        let prefix = format!(
            "{} {}/{}: ",
            format_timestamp_ms(entry.timestamp_ms),
            priority_label(entry.priority, entry.verbose_level),
            tag
        );
        for line in throwable.lines() {
            let _ = write!(out, "\n{prefix}{line}");
        }
    }
    out.push('\n');
    out
}

/// Formats one ToolPkg record with its package and script identifiers.
fn format_package_log_line(entry: &LogEntry) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{} {}/{} ",
        format_timestamp_ms(entry.timestamp_ms),
        priority_label(entry.priority, entry.verbose_level),
        TOOLPKG_LOG_TAG
    );
    if let Some(package_id) = extract_named_token(
        &entry.message,
        &["toolPkgId", "package", "subpackage", "container", "target"],
    ) {
        let _ = write!(out, "[PKG:{package_id}]");
    }
    if let Some(script_id) =
        extract_named_token(&entry.message, &["script", "path", "screen", "function"])
    {
        let _ = write!(out, "[SCRIPT:{script_id}]");
    }
    if let Some(plugin_id) = extract_named_token(&entry.message, &["plugin", "pluginId", "hookId"])
    {
        let _ = write!(out, "[PLUGIN:{plugin_id}]");
    }
    out.push(' ');
    out.push_str(&entry.message);
    if let Some(throwable) = &entry.throwable {
        let prefix = format!(
            "{} {}/{} ",
            format_timestamp_ms(entry.timestamp_ms),
            priority_label(entry.priority, entry.verbose_level),
            TOOLPKG_LOG_TAG
        );
        for line in throwable.lines() {
            let _ = write!(out, "\n{prefix}{line}");
        }
    }
    out.push('\n');
    out
}

/// The device uses UTC clock fields directly, avoiding the desktop local-time
/// formatter and timezone machinery in the fixed 4 MB firmware image.
#[cfg(target_os = "espidf")]
fn format_timestamp_ms(timestamp_ms: u128) -> String {
    let day = timestamp_ms % 86_400_000;
    format!("{:02}:{:02}:{:02}.{:03}", day / 3_600_000, day / 60_000 % 60, day / 1_000 % 60, day % 1_000)
}

/// Formats one epoch-millis timestamp for human-readable local diagnostics.
#[cfg(not(target_os = "espidf"))]
fn format_timestamp_ms(timestamp_ms: u128) -> String {
    let timestamp = timestamp_ms.min(i64::MAX as u128) as i64;
    let Some(datetime) = DateTime::<Utc>::from_timestamp_millis(timestamp) else {
        return timestamp_ms.to_string();
    };
    datetime
        .with_timezone(&Local)
        .format("%H:%M:%S%.3f")
        .to_string()
}

/// Returns the display character for one log priority.
fn priority_char(priority: i32) -> char {
    match priority {
        VERBOSE => 'V',
        DEBUG => 'D',
        INFO => 'I',
        WARN => 'W',
        ERROR => 'E',
        ASSERT => 'A',
        _ => '?',
    }
}

/// Formats a severity code with the Verbose detail level when present.
fn priority_label(priority: i32, verbose_level: Option<i32>) -> String {
    if priority == VERBOSE {
        if let Some(verbose_level) = verbose_level {
            return format!("V{verbose_level}");
        }
    }
    priority_char(priority).to_string()
}

/// Returns whether one log entry passes both console thresholds.
fn is_console_loggable(
    priority: i32,
    verbose_level: Option<i32>,
    console_min_level: i32,
    console_max_verbose_level: i32,
) -> bool {
    if priority < console_min_level {
        return false;
    }
    if priority == VERBOSE {
        return verbose_level.is_some_and(|level| level <= console_max_verbose_level);
    }
    true
}

/// Reads the first configured name-value token from a log message.
fn extract_named_token(text: &str, names: &[&str]) -> Option<String> {
    for name in names {
        let marker = format!("{name}=");
        if let Some(start) = text.find(&marker) {
            let value_start = start + marker.len();
            let value = text[value_start..]
                .split(|ch: char| ch.is_whitespace() || ch == ',')
                .next()
                .unwrap_or("")
                .trim_matches('"')
                .trim_matches('\'')
                .trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Formats an error and its source chain as multiline text.
fn error_chain(error: &(dyn std::error::Error)) -> String {
    let mut out = error.to_string();
    let mut source = error.source();
    while let Some(err) = source {
        out.push_str("\ncaused by: ");
        out.push_str(&err.to_string());
        source = err.source();
    }
    out
}

#[cfg(test)]
#[path = "AppLoggerTests.rs"]
mod tests;
