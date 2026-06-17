#![allow(non_snake_case)]

#[path = "../../common/external_event.rs"]
pub mod external_event;

mod filesystem;
mod http;
mod managed_runtime;
mod runtime_common;
mod runtime_storage;
mod system_operation;
mod terminal;
mod web_visit;

pub use external_event::LocalExternalRuntimeEventHost as AndroidExternalRuntimeEventHost;
pub use filesystem::AndroidFileSystemHost;
pub use http::AndroidHttpHost;
pub use managed_runtime::AndroidManagedRuntimeHost;
pub use runtime_storage::AndroidRuntimeStorageHost;
pub use system_operation::AndroidSystemOperationHost;
pub use terminal::AndroidTerminalHost;
pub use web_visit::AndroidWebVisitHost;
