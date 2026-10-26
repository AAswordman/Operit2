//! Windows daemon transport, private files, background launch, and termination signals.

use std::ffi::c_void;
use std::fs::{self, OpenOptions};
use std::io;
use std::mem::size_of;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::ptr;

use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::{LocalFree, ERROR_PIPE_BUSY};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenUser, SECURITY_ATTRIBUTES, SECURITY_MAX_SID_SIZE, TOKEN_QUERY,
    TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcessToken, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS,
};

use operit_host_native_common::DaemonHost::ControlStream;

/// Resolves the identity's local named pipe without using filesystem socket probes.
pub fn control_endpoint(_directory: &Path, identity_hash: &str) -> PathBuf {
    PathBuf::from(format!(r"\\.\pipe\operit2-{identity_hash}-control"))
}

/// Creates daemon directories beneath the identity's Windows storage root.
pub fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

/// Uses the identity directory's inherited Windows permissions for daemon files.
pub fn private_open_options() -> OpenOptions {
    OpenOptions::new()
}

/// Leaves named pipe lifetime to the kernel and its open handles.
pub fn cleanup_endpoint(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Releases a security API allocation with the allocator that created it.
struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
    /// Releases the Windows local allocation exactly once.
    fn drop(&mut self) {
        unsafe { LocalFree(self.0) };
    }
}

#[repr(C)]
struct TokenUserBuffer {
    user: TOKEN_USER,
    sid: [u8; SECURITY_MAX_SID_SIZE as usize],
}

/// Reads the process user's SID for an explicit owner-only pipe DACL.
fn current_user_sid() -> io::Result<String> {
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut buffer: TokenUserBuffer = unsafe { std::mem::zeroed() };
    let mut length = 0;
    if unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            &mut buffer as *mut _ as *mut c_void,
            size_of::<TokenUserBuffer>() as u32,
            &mut length,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(buffer.user.User.Sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let allocation = LocalAllocation(text.cast());
    let mut count = 0;
    while unsafe { *text.add(count) } != 0 {
        count += 1;
    }
    let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(text, count) })
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    drop(allocation);
    Ok(sid)
}

/// Creates a local-only pipe instance with access restricted to the process user.
fn create_pipe(path: &Path, first: bool) -> io::Result<NamedPipeServer> {
    let sid = current_user_sid()?;
    let sddl: Vec<u16> = format!("D:P(A;;GA;;;{sid})")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let allocation = LocalAllocation(descriptor);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let server = unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(path, &mut attributes as *mut _ as *mut c_void)
    };
    drop(allocation);
    server
}

/// Owns the pending pipe instance while accepted clients execute independently.
pub struct ControlListener {
    path: PathBuf,
    pending: NamedPipeServer,
}

impl ControlListener {
    /// Claims the pipe name exclusively for this identity's Core owner.
    pub fn bind(path: &Path) -> io::Result<Self> {
        Ok(Self {
            path: path.to_owned(),
            pending: create_pipe(path, true)?,
        })
    }

    /// Arms the next instance before handing an accepted connection to Core.
    pub async fn accept(&mut self) -> io::Result<ControlStream> {
        self.pending.connect().await?;
        let next = create_pipe(&self.path, false)?;
        let connected = std::mem::replace(&mut self.pending, next);
        Ok(Box::new(connected))
    }
}

/// Waits for an available instance of the same named pipe under the caller's timeout.
pub async fn connect(path: &Path) -> io::Result<ControlStream> {
    loop {
        match ClientOptions::new().open(path) {
            Ok(client) => return Ok(Box::new(client)),
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

/// Starts the daemon without a console or attachment to the invoking process group.
pub fn spawn_detached(command: &mut Command) -> io::Result<Child> {
    command
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
}

/// Waits for Windows console close, logoff, or system shutdown notifications.
pub async fn termination_signal() -> io::Result<()> {
    let mut close = tokio::signal::windows::ctrl_close()?;
    let mut logoff = tokio::signal::windows::ctrl_logoff()?;
    let mut shutdown = tokio::signal::windows::ctrl_shutdown()?;
    tokio::select! {
        _ = close.recv() => Ok(()),
        _ = logoff.recv() => Ok(()),
        _ = shutdown.recv() => Ok(()),
    }
}

#[cfg(test)]
#[path = "../../tests/daemon.rs"]
mod transport_tests;

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    use windows_sys::Win32::System::Console::GetConsoleWindow;

    use super::transport_tests::TestEndpoint;
    use super::{connect, spawn_detached, termination_signal, ControlListener};

    /// Checks that an existing named pipe cannot be taken over by another listener.
    #[tokio::test]
    async fn first_pipe_instance_rejects_another_listener() {
        let endpoint = TestEndpoint::new();
        let listener = ControlListener::bind(&endpoint.0).expect("owner bind");
        assert!(ControlListener::bind(&endpoint.0).is_err());
        drop(listener);
    }

    /// Checks that busy pipe waits yield to the executor and respect caller deadlines.
    #[tokio::test]
    async fn busy_pipe_wait_is_cancellable() {
        let endpoint = TestEndpoint::new();
        let mut listener = ControlListener::bind(&endpoint.0).expect("bind");
        let first = connect(&endpoint.0).await.expect("first client");
        assert!(
            tokio::time::timeout(Duration::from_millis(80), connect(&endpoint.0))
                .await
                .is_err()
        );
        let accepted = listener.accept().await.expect("accept first client");
        let second = connect(&endpoint.0).await.expect("next instance");
        drop((first, accepted, second, listener));
    }

    /// Re-executes this probe through the production launcher and checks its console isolation.
    #[test]
    fn detached_process_has_no_console() {
        const PROBE: &str = "OPERIT_DAEMON_DETACHED_CHILD_PROBE";
        if std::env::var_os(PROBE) == Some(OsString::from("1")) {
            assert!(
                unsafe { GetConsoleWindow() }.is_null(),
                "detached child acquired a console"
            );
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("probe runtime");
            runtime.block_on(async {
                let signal = tokio::time::timeout(Duration::from_millis(100), async {
                    tokio::select! {
                        result = tokio::signal::ctrl_c() => result,
                        result = termination_signal() => result,
                    }
                })
                .await;
                assert!(
                    signal.is_err(),
                    "detached signal registration failed or terminated: {signal:?}"
                );
            });
            return;
        }
        let mut command = Command::new(std::env::current_exe().expect("test executable"));
        command
            .args(["--exact", "daemon::tests::detached_process_has_no_console"])
            .env(PROBE, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = spawn_detached(&mut command)
            .expect("detached spawn")
            .wait_with_output()
            .expect("probe exit");
        assert!(
            output.status.success(),
            "detached probe failed: {}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
