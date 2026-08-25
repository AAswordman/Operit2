use std::backtrace::Backtrace;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, Once, OnceLock};

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

const TOOLPKG_LOG_TAG: &str = "ToolPkg";

#[derive(Debug, Clone, Serialize, Deserialize)]
/// One in-memory and file-backed application log entry.
pub struct LogEntry {
    pub priority: i32,
    pub tag: String,
    pub message: String,
    pub throwable: Option<String>,
    pub timestamp_ms: u128,
}

struct LoggerState {
    enable_file_logging: bool,
    enable_console_logging: bool,
    file_system_host: Option<Arc<dyn FileSystemHost>>,
    log_file: Option<String>,
    package_log_file: Option<String>,
    entries: Vec<LogEntry>,
}

static STATE: OnceLock<Mutex<LoggerState>> = OnceLock::new();
static HOST_LOG_SINK_INIT: Once = Once::new();

fn state() -> &'static Mutex<LoggerState> {
    install_host_log_sink_once();
    STATE.get_or_init(|| {
        Mutex::new(LoggerState {
            enable_file_logging: true,
            enable_console_logging: true,
            file_system_host: None,
            log_file: None,
            package_log_file: None,
            entries: Vec::new(),
        })
    })
}

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

    /// Configures runtime and ToolPkg logs through the supplied file-system host.
    pub fn configure_log_files(
        file_system_host: Arc<dyn FileSystemHost>,
        log_file: String,
        package_log_file: String,
    ) -> Result<(), String> {
        ensure_log_file(&file_system_host, &log_file)?;
        ensure_log_file(&file_system_host, &package_log_file)?;
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
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
        file_system_host
            .readFile(&path)
            .map_err(|error| error.message)
    }

    /// Reads the ToolPkg log file as text.
    pub fn package_text() -> Result<String, String> {
        let (file_system_host, path) = log_file_access(true)?;
        file_system_host
            .readFile(&path)
            .map_err(|error| error.message)
    }

    /// Writes a verbose log message.
    pub fn v(tag: &str, msg: &str) -> i32 {
        Self::println(VERBOSE, tag, msg)
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
        write_entry(priority, tag, msg, None);
        0
    }

    /// Writes a log message together with an error chain.
    pub fn println_with_error(
        priority: i32,
        tag: &str,
        msg: &str,
        tr: &(dyn std::error::Error),
    ) -> i32 {
        write_entry(priority, tag, msg, Some(error_chain(tr)));
        0
    }

    /// Captures the current stack trace as display text.
    pub fn get_stack_trace_string(_tr: &(dyn std::error::Error)) -> String {
        format!("{:?}", Backtrace::capture())
    }

    /// Returns whether a tag and priority should be logged.
    pub fn is_loggable(_tag: &str, _level: i32) -> bool {
        true
    }
}

fn write_entry(priority: i32, tag: &str, msg: &str, throwable: Option<String>) {
    let timestamp_ms = operit_host_api::TimeUtils::currentTimeMillisU128();
    let entry = LogEntry {
        priority,
        tag: tag.to_string(),
        message: msg.to_string(),
        throwable,
        timestamp_ms,
    };

    let (enable_file_logging, enable_console_logging, file_system_host, log_file, package_log_file) = {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.entries.push(entry.clone());
        (
            guard.enable_file_logging,
            guard.enable_console_logging,
            guard.file_system_host.clone(),
            guard.log_file.clone(),
            guard.package_log_file.clone(),
        )
    };

    let line = format_log_line(&entry, tag);
    if enable_console_logging {
        match priority {
            ERROR | ASSERT => eprint!("{line}"),
            _ => print!("{line}"),
        }
    }

    if enable_file_logging {
        if let Some(file_system_host) = file_system_host {
            if let Some(path) = log_file {
                append_line(&file_system_host, &path, &line);
            }
            if tag.eq_ignore_ascii_case(TOOLPKG_LOG_TAG) {
                if let Some(path) = package_log_file {
                    append_line(&file_system_host, &path, &format_package_log_line(&entry));
                }
            }
        }
    }
}

/// Appends one formatted log line through the configured file-system host.
fn append_line(file_system_host: &Arc<dyn FileSystemHost>, path: &str, line: &str) {
    let _ = file_system_host.writeFile(path, line, true);
}

/// Creates an empty log file through the host when the path is absent.
fn ensure_log_file(file_system_host: &Arc<dyn FileSystemHost>, path: &str) -> Result<(), String> {
    let metadata = file_system_host
        .fileExists(path)
        .map_err(|error| error.message)?;
    if !metadata.exists {
        file_system_host
            .writeFile(path, "", false)
            .map_err(|error| error.message)?;
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

fn format_log_line(entry: &LogEntry, tag: &str) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{} {}/{}: {}",
        format_timestamp_ms(entry.timestamp_ms),
        priority_char(entry.priority),
        tag,
        entry.message
    );
    if let Some(throwable) = &entry.throwable {
        let prefix = format!(
            "{} {}/{}: ",
            format_timestamp_ms(entry.timestamp_ms),
            priority_char(entry.priority),
            tag
        );
        for line in throwable.lines() {
            let _ = write!(out, "\n{prefix}{line}");
        }
    }
    out.push('\n');
    out
}

fn format_package_log_line(entry: &LogEntry) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{} {}/{} ",
        format_timestamp_ms(entry.timestamp_ms),
        priority_char(entry.priority),
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
            priority_char(entry.priority),
            TOOLPKG_LOG_TAG
        );
        for line in throwable.lines() {
            let _ = write!(out, "\n{prefix}{line}");
        }
    }
    out.push('\n');
    out
}

/// Formats one epoch-millis timestamp for human-readable local diagnostics.
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
