use std::backtrace::Backtrace;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub const VERBOSE: i32 = 2;
pub const DEBUG: i32 = 3;
pub const INFO: i32 = 4;
pub const WARN: i32 = 5;
pub const ERROR: i32 = 6;
pub const ASSERT: i32 = 7;

const TOOLPKG_LOG_TAG: &str = "ToolPkg";

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub priority: i32,
    pub tag: String,
    pub message: String,
    pub throwable: Option<String>,
    pub timestamp_ms: u128,
}

#[derive(Debug, Default)]
struct LoggerState {
    enable_file_logging: bool,
    log_file: Option<PathBuf>,
    package_log_file: Option<PathBuf>,
    entries: Vec<LogEntry>,
}

static STATE: OnceLock<Mutex<LoggerState>> = OnceLock::new();

fn state() -> &'static Mutex<LoggerState> {
    STATE.get_or_init(|| {
        Mutex::new(LoggerState {
            enable_file_logging: true,
            log_file: None,
            package_log_file: None,
            entries: Vec::new(),
        })
    })
}

pub struct AppLogger;

impl AppLogger {
    pub fn set_enable_file_logging(enabled: bool) {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.enable_file_logging = enabled;
    }

    pub fn enable_file_logging() -> bool {
        state().lock().expect("AppLogger mutex poisoned").enable_file_logging
    }

    pub fn bind_log_file(path: impl Into<PathBuf>) {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.log_file = Some(path.into());
    }

    pub fn bind_package_log_file(path: impl Into<PathBuf>) {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.package_log_file = Some(path.into());
    }

    pub fn get_log_file() -> Option<PathBuf> {
        state().lock().expect("AppLogger mutex poisoned").log_file.clone()
    }

    pub fn reset_log_file() {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        if let Some(path) = &guard.log_file {
            let _ = fs::remove_file(path);
        }
        if let Some(path) = &guard.package_log_file {
            let _ = fs::remove_file(path);
        }
        guard.entries.clear();
    }

    pub fn entries() -> Vec<LogEntry> {
        state().lock().expect("AppLogger mutex poisoned").entries.clone()
    }

    pub fn v(tag: &str, msg: &str) -> i32 {
        Self::println(VERBOSE, tag, msg)
    }

    pub fn d(tag: &str, msg: &str) -> i32 {
        Self::println(DEBUG, tag, msg)
    }

    pub fn i(tag: &str, msg: &str) -> i32 {
        Self::println(INFO, tag, msg)
    }

    pub fn w(tag: &str, msg: &str) -> i32 {
        Self::println(WARN, tag, msg)
    }

    pub fn e(tag: &str, msg: &str) -> i32 {
        Self::println(ERROR, tag, msg)
    }

    pub fn wtf(tag: &str, msg: &str) -> i32 {
        Self::println(ASSERT, tag, msg)
    }

    pub fn println(priority: i32, tag: &str, msg: &str) -> i32 {
        write_entry(priority, tag, msg, None);
        0
    }

    pub fn println_with_error(priority: i32, tag: &str, msg: &str, tr: &(dyn std::error::Error)) -> i32 {
        write_entry(priority, tag, msg, Some(error_chain(tr)));
        0
    }

    pub fn get_stack_trace_string(_tr: &(dyn std::error::Error)) -> String {
        format!("{:?}", Backtrace::capture())
    }

    pub fn is_loggable(_tag: &str, _level: i32) -> bool {
        true
    }
}

fn write_entry(priority: i32, tag: &str, msg: &str, throwable: Option<String>) {
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let entry = LogEntry {
        priority,
        tag: tag.to_string(),
        message: msg.to_string(),
        throwable,
        timestamp_ms,
    };

    let (enable_file_logging, log_file, package_log_file) = {
        let mut guard = state().lock().expect("AppLogger mutex poisoned");
        guard.entries.push(entry.clone());
        (
            guard.enable_file_logging,
            guard.log_file.clone(),
            guard.package_log_file.clone(),
        )
    };

    let line = format_log_line(&entry, tag);
    match priority {
        ERROR | ASSERT => eprint!("{line}"),
        _ => print!("{line}"),
    }

    if enable_file_logging {
        if let Some(path) = log_file {
            append_line(&path, &line);
        }
        if tag.eq_ignore_ascii_case(TOOLPKG_LOG_TAG) {
            if let Some(path) = package_log_file {
                append_line(&path, &format_package_log_line(&entry));
            }
        }
    }
}

fn append_line(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn format_log_line(entry: &LogEntry, tag: &str) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{} {}/{}: {}",
        entry.timestamp_ms,
        priority_char(entry.priority),
        tag,
        entry.message
    );
    if let Some(throwable) = &entry.throwable {
        let _ = write!(out, "\n{throwable}");
    }
    out.push('\n');
    out
}

fn format_package_log_line(entry: &LogEntry) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{} {}/{} ",
        entry.timestamp_ms,
        priority_char(entry.priority),
        TOOLPKG_LOG_TAG
    );
    if let Some(package_id) = extract_named_token(&entry.message, &["toolPkgId", "package", "subpackage", "container", "target"]) {
        let _ = write!(out, "[PKG:{package_id}]");
    }
    if let Some(script_id) = extract_named_token(&entry.message, &["script", "path", "screen", "function"]) {
        let _ = write!(out, "[SCRIPT:{script_id}]");
    }
    if let Some(plugin_id) = extract_named_token(&entry.message, &["plugin", "pluginId", "hookId"]) {
        let _ = write!(out, "[PLUGIN:{plugin_id}]");
    }
    out.push(' ');
    out.push_str(&entry.message);
    if let Some(throwable) = &entry.throwable {
        let _ = write!(out, "\n{throwable}");
    }
    out.push('\n');
    out
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
