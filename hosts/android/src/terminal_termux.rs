//! Launches a Termux-owned PTY and transfers its streams over authenticated loopback sockets.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use operit_host_api::{HostError, HostResult};
use serde_json::Value;

pub(crate) const HOME: &str = "/data/data/com.termux/files/home";
pub type TermuxLauncher = Arc<dyn Fn(Value) -> HostResult<()> + Send + Sync>;

pub(crate) struct TermuxControl {
    socket: TcpStream,
    exit: [u8; 4],
    received: usize,
    outputClosed: Arc<AtomicBool>,
}

impl TermuxControl {
    /// Stops the remote PTY when its owning session closes.
    pub(crate) fn close(&self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }

    /// Marks when the existing host reader has published the final PTY bytes.
    pub(crate) fn outputClosed(&self) -> Arc<AtomicBool> {
        self.outputClosed.clone()
    }

    /// Forwards a terminal size change to the Termux-owned master PTY.
    pub(crate) fn resize(&mut self, rows: u16, cols: u16) -> HostResult<()> {
        let [rowHigh, rowLow] = rows.to_be_bytes();
        let [colHigh, colLow] = cols.to_be_bytes();
        if let Err(error) = self.socket.write_all(&[rowHigh, rowLow, colHigh, colLow]) {
            let _ = self.socket.shutdown(Shutdown::Both);
            return Err(error.into());
        }
        Ok(())
    }

    /// Reads the actual shell exit status without blocking the terminal output pump.
    pub(crate) fn pollExit(&mut self) -> HostResult<Option<i32>> {
        if !self.outputClosed.load(Ordering::Acquire) {
            return Ok(None);
        }
        while self.received < self.exit.len() {
            match self.socket.read(&mut self.exit[self.received..]) {
                Ok(0) => return Err(HostError::new("Termux disconnected before reporting its exit status")),
                Ok(count) => self.received += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(Some(i32::from_be_bytes(self.exit)))
    }
}

#[cfg(target_os = "android")]
/// Starts the app-owned intent, then authenticates the PTY data and control connections.
pub(crate) fn start(
    launch: &TermuxLauncher,
    prompt: &str,
    directory: &str,
    rows: u16,
    cols: u16,
) -> HostResult<(TermuxControl, i32, i32)> {
    use std::net::{Ipv4Addr, TcpListener};
    use std::os::fd::{AsRawFd, IntoRawFd};
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    let token = uuid::Uuid::new_v4().simple().to_string();
    launch(serde_json::json!({
        "port": port, "token": token, "rows": rows, "cols": cols,
        "script": include_str!("terminal_termux.py"), "prompt": prompt, "directory": directory,
    }))?;

    let deadline = Instant::now() + Duration::from_secs(12);
    let mut data = None;
    let mut control = None;
    while data.is_none() || control.is_none() {
        let mut descriptor = libc::pollfd { fd: listener.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let timeout = deadline.saturating_duration_since(Instant::now());
        if timeout.is_zero() {
            return Err(termuxStartupTimeout());
        }
        let ready = unsafe { libc::poll(&mut descriptor, 1, timeout.as_millis().max(1) as i32) };
        if ready == 0 {
            return Err(termuxStartupTimeout());
        }
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted { continue; }
            return Err(error.into());
        }
        let (mut socket, _) = listener.accept()?;
        let mut hello = [0; 33];
        socket.set_read_timeout(Some(Duration::from_millis(500)))?;
        if socket.read_exact(&mut hello).is_err() || &hello[..32] != token.as_bytes() {
            continue;
        }
        socket.set_read_timeout(None)?;
        socket.set_nodelay(true)?;
        match hello[32] {
            b'D' if data.is_none() => data = Some(socket),
            b'C' if control.is_none() => control = Some(socket),
            _ => {}
        }
    }
    let data = data.expect("authenticated data socket");
    let control = control.expect("authenticated control socket");
    control.set_nonblocking(true)?;
    let writer = data.try_clone()?;
    Ok((TermuxControl {
        socket: control,
        exit: [0; 4],
        received: 0,
        outputClosed: Arc::new(AtomicBool::new(false)),
    }, data.into_raw_fd(), writer.into_raw_fd()))
}

#[cfg(target_os = "android")]
/// Explains the Termux setup required when its service could not open the session.
fn termuxStartupTimeout() -> HostError {
    HostError::new("Termux did not connect. Open Termux once, install Python with `pkg install --no-install-recommends python`, set `allow-external-apps = true` in ~/.termux/termux.properties, then run `termux-reload-settings`.")
}

#[cfg(not(target_os = "android"))]
/// Rejects Termux launches outside the Android target.
pub(crate) fn start(_: &TermuxLauncher, _: &str, _: &str, _: u16, _: u16) -> HostResult<(TermuxControl, i32, i32)> {
    Err(HostError::new("Termux is only available on Android"))
}
