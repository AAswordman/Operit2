#![cfg(unix)]

use operit_host_api::{HostRuntimeTaskSchedulerHost, TerminalHost};
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
use operit_host_native_terminal::NativePtyTerminalHost;
use std::collections::BTreeSet;
use std::sync::{mpsc, Arc};
use std::time::Duration;

/// Keeps more real shells waiting than either legacy worker pool could accommodate.
#[test]
fn silent_commands_release_workers_and_finish_on_input_events() {
    let host = Arc::new(NativePtyTerminalHost::new().withControllingTty(false));
    let scheduler = Arc::new(NativeHostRuntimeTaskSchedulerHost::new());
    let (started, starts) = mpsc::channel();
    let (completed, completions) = mpsc::channel();
    let mut sessions = Vec::new();
    for index in 0..32 {
        let session = host
            .createOrGetSession(&format!("cooperative-{index}"))
            .unwrap();
        let sessionId = session.sessionId.clone();
        sessions.push(sessionId.clone());
        let started = started.clone();
        let completed = completed.clone();
        let mut observed = false;
        host.clone()
            .scheduleInSessionStreaming(
                scheduler.clone(),
                sessionId,
                "read -r OPERIT_RELEASE".into(),
                0,
                Box::new(move |bytes, _| {
                    if !observed && !bytes.is_empty() {
                        observed = true;
                        started.send(index).unwrap();
                    }
                    Ok(())
                }),
                Box::new(move |receipt| {
                    completed.send((index, receipt)).unwrap();
                }),
            )
            .unwrap();
    }
    let mut running = BTreeSet::new();
    for _ in 0..32 {
        running.insert(
            starts
                .recv_timeout(Duration::from_secs(10))
                .expect("silent command did not start"),
        );
    }
    assert_eq!(running.len(), 32);
    assert!(host
        .listSessions()
        .unwrap()
        .iter()
        .all(|session| session.commandRunning));
    let (available, received) = mpsc::channel();
    scheduler
        .scheduleHostRuntimeTask(
            "terminal-worker-availability",
            Box::new(move || {
                available.send(()).unwrap();
            }),
        )
        .unwrap();
    received
        .recv_timeout(Duration::from_secs(5))
        .expect("silent commands exhausted blocking workers");
    for session in &sessions {
        host.inputInSession(session, Some("release\n"), None)
            .unwrap();
    }
    let mut finished = BTreeSet::new();
    for _ in 0..32 {
        let (index, receipt) = completions
            .recv_timeout(Duration::from_secs(10))
            .expect("input did not wake command completion");
        let receipt = receipt.unwrap();
        assert!(!receipt.timedOut);
        assert_eq!(receipt.exitCode, 0);
        finished.insert(index);
    }
    assert_eq!(finished.len(), 32);
    for session in sessions {
        host.closeSession(&session).unwrap();
    }
}
