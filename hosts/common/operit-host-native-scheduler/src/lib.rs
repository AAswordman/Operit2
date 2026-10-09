#![allow(non_snake_case)]

#[cfg(feature = "javascript")]
mod javascript_runtime;
#[cfg(feature = "javascript")]
mod javascript_values;
mod runtime_event_scheduler;
mod runtime_task_scheduler;

#[cfg(feature = "javascript")]
pub use javascript_runtime::NativeHostJavaScriptRuntimeHost;
pub use runtime_event_scheduler::NativeHostRuntimeEventSchedulerHost;
pub use runtime_task_scheduler::NativeHostRuntimeTaskSchedulerHost;

mod local_task_scheduler;
pub use local_task_scheduler::LocalHostRuntimeTaskSchedulerHost;
