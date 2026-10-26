//! Selects the daemon host supplied by the CLI platform package.

#[cfg(target_os = "linux")]
pub(crate) use operit_host_linux_native::daemon;
#[cfg(target_os = "macos")]
pub(crate) use operit_host_macos_native::daemon;
#[cfg(windows)]
pub(crate) use operit_host_windows_native::daemon;
