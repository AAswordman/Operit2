//! Checks the actual terminal adapters without linking application engines or Apple frameworks.
#![allow(dead_code, non_snake_case, unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../ios/src/bridge.rs"]
mod bridge;
#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../ios/src/terminal.rs"]
mod ios_terminal;

#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../common/terminal_command.rs"]
mod terminal_command;
#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../android/src/terminal.rs"]
mod android_terminal;
#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../android/src/runtime_common.rs"]
mod runtime_common;
#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../android/src/terminal_termux.rs"]
mod terminal_termux;
#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../android/src/terminal_privilege.rs"]
mod terminal_privilege;
#[cfg(not(target_arch = "wasm32"))]
#[path = "../../../android/src/document_filesystem.rs"]
mod document_filesystem;

#[cfg(target_os = "android")]
#[path = "../../../android/src/secret_store.rs"]
mod secret_store;
#[cfg(target_os = "android")]
#[path = "../../../android/src/jni_thread.rs"]
mod jni_thread;
#[cfg(target_os = "android")]
#[path = "../../../android/src/runtime_event_scheduler.rs"]
mod runtime_event_scheduler;
#[cfg(target_os = "android")]
pub use jni_thread::attachCurrentThread;

#[cfg(windows)]
pub use operit_host_native_common::NativeHttpHost as WindowsHttpHost;
#[cfg(windows)]
#[path = "../../../windows/src/tools/terminal/mod.rs"]
mod windows_terminal;

#[cfg(windows)]
#[path = "../../../windows/src/process.rs"]
mod process;

#[cfg(target_os = "linux")]
#[path = "../../../linux/src/tools/terminal/mod.rs"]
mod linux_terminal;

#[cfg(test)]
mod process_sessions;

#[cfg(target_arch = "wasm32")]
#[path = "../../../web/src/common.rs"]
mod common;

#[cfg(target_arch = "wasm32")]
#[path = "../../../web/src/tools/terminal/mod.rs"]
mod web_terminal;
