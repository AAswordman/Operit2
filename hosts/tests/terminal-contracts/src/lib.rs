//! Checks the actual terminal adapters without linking application engines or Apple frameworks.
#![allow(dead_code, non_snake_case, unused_imports)]

#[path = "../../../ios/src/bridge.rs"]
mod bridge;
#[path = "../../../ios/src/terminal.rs"]
mod ios_terminal;

#[cfg(windows)]
pub use operit_host_native_common::NativeHttpHost as WindowsHttpHost;
#[cfg(windows)]
#[path = "../../../windows/src/tools/terminal/mod.rs"]
mod windows_terminal;

#[cfg(target_os = "linux")]
#[path = "../../../linux/src/tools/terminal/mod.rs"]
mod linux_terminal;
