//! Real-shell checks for process completion, literal input and persistent interaction.
#![cfg(any(windows, target_os = "linux"))]

use operit_host_api::{HostRuntimeTaskSchedulerHost, TerminalHost, TerminalSessionRequest};
use std::sync::{mpsc, Arc};
use std::time::Duration;

/// Selects the production terminal adapter used by this fixture's host.
fn host() -> Arc<dyn TerminalHost> {
    #[cfg(windows)]
    { Arc::new(crate::windows_terminal::WindowsTerminalHost::new()) }
    #[cfg(target_os = "linux")]
    { Arc::new(crate::linux_terminal::LinuxTerminalHost::new()) }
}

/// Starts a real raw session with a complete script supplied as an argument.
fn open(host: &dyn TerminalHost, kind: &str, script: Option<&str>) -> String {
    host.openTerminalSession(&TerminalSessionRequest {
        sessionName: uuid::Uuid::new_v4().to_string(), terminal: "native".into(),
        terminalType: kind.into(), workingDir: Some(std::env::temp_dir().to_string_lossy().into_owned()),
        command: script.map(str::to_string),
    }).unwrap().sessionId
}

/// Observes actual process completion on the same cooperative scheduler used in production.
fn observe(host: Arc<dyn TerminalHost>, id: String) -> mpsc::Receiver<(operit_host_api::HostResult<i32>, Vec<u8>)> {
    let scheduler = Arc::new(operit_host_native_common::NativeHostRuntimeTaskSchedulerHost::new());
    let clock = scheduler.clone();
    let (send, receive) = mpsc::channel();
    scheduler.scheduleHostRuntimeCooperativeAsyncTask("test-process-exit", Box::new(move || Box::pin(async move {
        let bytes = Arc::new(std::sync::Mutex::new(Vec::new()));
        let output = bytes.clone();
        let result = operit_host_api::TerminalCommand::observeTerminalProcess(host, clock, id,
            Box::new(move |chunk, _| { output.lock().unwrap().extend_from_slice(chunk); Ok(()) })).await;
        send.send((result, bytes.lock().unwrap().clone())).unwrap();
    }))).unwrap();
    receive
}

/// A script without a trailing newline starts immediately and returns its real nonzero status.
#[test]
fn process_script_does_not_need_enter_or_a_prompt_marker() {
    let host = host();
    let id = open(&*host, "bash", Some("printf '雪🌍'; exit 7"));
    let (code, output) = observe(host.clone(), id.clone()).recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(code.unwrap(), 7);
    assert!(String::from_utf8_lossy(&output).contains("雪🌍"));
    assert!(!host.getSessionScreen(&id).unwrap().commandRunning);
    host.closeSession(&id).unwrap();
}

/// Syntax errors, silence and redirected output cannot hide process exit from the observer.
#[test]
fn malformed_silent_and_redirected_scripts_finish() {
    let host = host();
    for (script, expected) in [("if then", 2), ("exit 9", 9), ("exec >/dev/null 2>&1; exit 3", 3)] {
        let id = open(&*host, "bash", Some(script));
        let (code, _) = observe(host.clone(), id.clone()).recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(code.unwrap(), expected, "{script}");
        host.closeSession(&id).unwrap();
    }
}

/// Waiting for process exit never reserves input or screen access to the same PTY.
#[test]
fn running_process_accepts_input_while_observed() {
    let host = host();
    let id = open(&*host, "bash", Some("read -r answer; printf 'answer:%s' \"$answer\""));
    let result = observe(host.clone(), id.clone());
    assert!(result.recv_timeout(Duration::from_millis(150)).is_err());
    assert!(host.getSessionScreen(&id).unwrap().commandRunning);
    host.inputInSession(&id, Some("hello"), Some("enter")).unwrap();
    let (code, output) = result.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(code.unwrap(), 0);
    assert!(String::from_utf8_lossy(&output).contains("answer:hello"));
    host.closeSession(&id).unwrap();
}

/// A command larger than a canonical terminal input line bypasses the line editor entirely.
#[test]
fn long_scripts_bypass_terminal_line_limits() {
    let host = host();
    let script = format!("# {}\nprintf long-script-completed", "x".repeat(12000));
    let id = open(&*host, "bash", Some(&script));
    let (code, output) = observe(host.clone(), id.clone()).recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(code.unwrap(), 0);
    assert!(String::from_utf8_lossy(&output).contains("long-script-completed"), "output={:?}", String::from_utf8_lossy(&output));
    host.closeSession(&id).unwrap();
}

/// A persistent interactive shell keeps its variables and cannot be reclaimed as an idle command.
#[test]
fn interactive_shell_keeps_context_until_explicit_close() {
    let host = host();
    let id = open(&*host, "bash", None);
    let name = host.listSessions().unwrap().into_iter().find(|entry| entry.sessionId == id).unwrap().sessionName;
    assert!(host.closeSessionIfIdle(&id, &name).is_err());
    host.inputInSession(&id, Some("export OPERIT_INTERACTIVE=kept"), Some("enter")).unwrap();
    host.inputInSession(&id, Some("printf 'context:%s' \"$OPERIT_INTERACTIVE\"; exit"), Some("enter")).unwrap();
    let (code, output) = observe(host.clone(), id.clone()).recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(code.unwrap(), 0);
    assert!(String::from_utf8_lossy(&output).contains("context:kept"));
    host.closeSession(&id).unwrap();
}

/// Ctrl+C is a control byte, never a literal 'c' followed by an interrupt.
#[test]
fn interrupt_is_not_typed_as_text_and_close_releases_waiters() {
    let host = host();
    let id = open(&*host, "bash", Some("read -r answer"));
    let result = observe(host.clone(), id.clone());
    assert!(result.recv_timeout(Duration::from_millis(150)).is_err());
    host.inputInSession(&id, Some("c"), Some("ctrl")).unwrap();
    let (code, output) = result.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_ne!(code.unwrap(), 0);
    assert!(!String::from_utf8_lossy(&output).contains("c^C"));
    host.closeSession(&id).unwrap();
    let id = open(&*host, "bash", Some("read -r answer"));
    let result = observe(host.clone(), id.clone());
    host.closeSession(&id).unwrap();
    assert!(result.recv_timeout(Duration::from_secs(10)).unwrap().0.is_err());
}

/// PowerShell uses the process status even when no prompt or output is produced.
#[cfg(windows)]
#[test]
fn powershell_process_reports_real_exit_and_parser_failure() {
    let host = host();
    for (script, expected) in [("exit 12", 12), ("if (", 1), ("Write-Output '中文'; exit 0", 0)] {
        let id = open(&*host, "powershell", Some(script));
        let (code, _) = observe(host.clone(), id.clone()).recv_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(code.unwrap(), expected);
        host.closeSession(&id).unwrap();
    }
}
