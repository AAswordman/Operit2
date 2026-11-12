#![cfg(unix)]
use operit_host_api::TerminalHost;
use operit_host_native_terminal::{NativePtyShellCommand, NativePtyTerminalHost};
use std::time::{Duration, Instant};

fn fd_count() -> usize {
    std::fs::read_dir(if cfg!(target_os = "linux") {
        "/proc/self/fd"
    } else {
        "/dev/fd"
    })
    .unwrap()
    .count()
}
fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn terminal_fd_lifecycle() {
    if std::env::var_os("OPERIT_TERMINAL_FD_TEST_CHILD").is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "terminal_fd_lifecycle", "--nocapture"])
            .env("OPERIT_TERMINAL_FD_TEST_CHILD", "1")
            .current_dir("/")
            .status()
            .unwrap()
            .success());
        return;
    }
    let host = NativePtyTerminalHost::new().withControllingTty(false);
    let warmup = host.createOrGetSession("warmup").unwrap();
    host.closePtySession(&warmup.sessionId).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let baseline = fd_count();

    // Retain a slave to model a descendant without leaving an actual background process.
    for index in 0..24 {
        let session = host
            .createOrGetSession(&format!("background-{index}"))
            .unwrap();
        let tty = host
            .executeInSession(&session.sessionId, "tty", 3000)
            .unwrap();
        let path = tty
            .output
            .lines()
            .find(|line| line.starts_with("/dev/"))
            .unwrap();
        let slave = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();
        host.closePtySession(&session.sessionId).unwrap();
        assert!(
            fd_count() <= baseline + 1,
            "PTY handles retained after close"
        );
        drop(slave);
    }
    // Both polling and session listing must reclaim handles while retaining output and status.
    for index in 0..32 {
        let session = host.createOrGetSession(&format!("exited-{index}")).unwrap();
        host.writePtySession(&session.sessionId, b"printf 'last-output\\n'; exit 7\r")
            .unwrap();
        wait_until(|| {
            if index < 24 {
                host.pollPtyExitCode(&session.sessionId).unwrap() == Some(7)
            } else {
                !host
                    .listSessions()
                    .unwrap()
                    .iter()
                    .any(|entry| entry.sessionId == session.sessionId)
            }
        });
        assert_eq!(host.pollPtyExitCode(&session.sessionId).unwrap(), Some(7));
        assert!(host.ownsSession(&session.sessionId).unwrap());
        assert!(
            String::from_utf8_lossy(&host.readPtySession(&session.sessionId).unwrap())
                .contains("last-output")
        );
        assert!(fd_count() <= baseline, "PTY handles retained after exit");
    }
    let broken = NativePtyTerminalHost::customShell(NativePtyShellCommand {
        program: "/bin/sh".into(),
        arguments: vec![
            "-c".into(),
            "printf '\\033]133;OperitPrompt=invalid\\007'; exec sleep 10".into(),
        ],
        description: "invalid initial prompt".into(),
        processWorkingDirectory: "/tmp".into(),
        defaultSessionWorkingDirectory: "/tmp".into(),
        environment: vec![],
        sessionWorkingDirectoryEnvironment: "OPERIT_TEST_CWD".into(),
    })
    .unwrap()
    .withControllingTty(false);
    for index in 0..8 {
        let error = broken
            .createOrGetSession(&format!("failed-{index}"))
            .unwrap_err();
        assert!(
            error.to_string().contains("Invalid PTY prompt marker"),
            "{error}"
        );
        assert!(
            fd_count() <= baseline,
            "PTY handles retained after failed initialization"
        );
    }
    println!("FD lifecycle: baseline={baseline}, final={}", fd_count());
}
