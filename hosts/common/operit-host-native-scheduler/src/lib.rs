#![allow(non_snake_case)]

// Desktop hosts use the V8 engine; Android/iOS/OpenHarmony compile QuickJS.
#[cfg(all(
    feature = "javascript",
    not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
))]
mod javascript_engine;
#[cfg(all(
    feature = "javascript",
    any(target_os = "android", target_os = "ios", target_env = "ohos")
))]
#[path = "javascript_engine_quickjs.rs"]
mod javascript_engine;
#[cfg(all(
    feature = "javascript",
    not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
))]
mod javascript_interrupt;
#[cfg(all(
    feature = "javascript",
    not(any(target_os = "android", target_os = "ios", target_env = "ohos"))
))]
mod javascript_values;
#[cfg(all(
    feature = "javascript",
    any(target_os = "android", target_os = "ios", target_env = "ohos")
))]
#[path = "javascript_values_quickjs.rs"]
mod javascript_values;
#[cfg(feature = "javascript")]
mod javascript_runtime;
mod runtime_event_scheduler;
mod runtime_task_scheduler;

#[cfg(feature = "javascript")]
pub use javascript_runtime::NativeHostJavaScriptRuntimeHost;
pub use runtime_event_scheduler::NativeHostRuntimeEventSchedulerHost;
pub use runtime_task_scheduler::NativeHostRuntimeTaskSchedulerHost;

mod local_task_scheduler;
pub use local_task_scheduler::LocalHostRuntimeTaskSchedulerHost;

mod task_handle;
pub use task_handle::{
    scheduleNativeBlockingTask, scheduleNativeServiceTask, scheduleNativeServiceTasks,
    NativeTaskHandle,
};
