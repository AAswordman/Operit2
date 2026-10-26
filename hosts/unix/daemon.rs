//! Unix daemon transport, private files, background launch, and termination signals.

use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use tokio::net::{UnixListener, UnixStream};

use operit_host_native_common::DaemonHost::ControlStream;

/// Resolves the identity's short Unix domain socket address.
pub fn control_endpoint(directory: &Path, _identity_hash: &str) -> PathBuf {
    directory.join("control.sock")
}

/// Creates a directory accessible only to its owning user.
pub fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

/// Creates file options with owner-only permissions for new daemon files.
pub fn private_open_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.mode(0o600);
    options
}

/// Removes the filesystem endpoint while leaving its private directory intact.
pub fn cleanup_endpoint(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Accepts owner-local control connections after the identity lock is acquired.
pub struct ControlListener {
    listener: UnixListener,
}

impl ControlListener {
    /// Replaces a stale socket under the exclusive identity owner lock.
    pub fn bind(path: &Path) -> io::Result<Self> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("control socket has no parent"))?;
        ensure_private_dir(parent)?;
        cleanup_endpoint(path)?;
        let listener = UnixListener::bind(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        Ok(Self { listener })
    }

    /// Accepts one connection without blocking the runtime executor.
    pub async fn accept(&mut self) -> io::Result<ControlStream> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Box::new(stream))
    }
}

/// Connects to an existing local Unix domain socket.
pub async fn connect(path: &Path) -> io::Result<ControlStream> {
    Ok(Box::new(UnixStream::connect(path).await?))
}

/// Starts a background process in its own process group.
pub fn spawn_detached(command: &mut Command) -> io::Result<Child> {
    command.process_group(0).spawn()
}

/// Waits for an orderly service termination request from the host.
pub async fn termination_signal() -> io::Result<()> {
    let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    signal
        .recv()
        .await
        .ok_or_else(|| io::Error::other("termination signal stream closed"))
}

#[cfg(test)]
#[path = "../tests/daemon.rs"]
mod transport_tests;
