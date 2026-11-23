#![allow(non_snake_case)]

#[cfg(feature = "javascript")]
mod javascript_engine;
#[cfg(feature = "javascript")]
mod javascript_interrupt;
#[cfg(feature = "javascript")]
mod javascript_runtime;
#[cfg(feature = "javascript")]
mod javascript_values;
mod runtime_event_scheduler;
mod runtime_task_scheduler;
mod task_poll_monitor;
mod terminal_command;

#[cfg(feature = "javascript")]
pub use javascript_runtime::NativeHostJavaScriptRuntimeHost;
pub use runtime_event_scheduler::NativeHostRuntimeEventSchedulerHost;
pub use runtime_task_scheduler::NativeHostRuntimeTaskSchedulerHost;
pub use terminal_command::{runTerminalCommandSynchronously, waitForTerminalPrompt};

#[cfg(feature = "esp32-compat")]
mod stack_budget;
#[cfg(feature = "esp32-compat")]
pub use stack_budget::withStackBudget;

mod local_task_scheduler;
pub use local_task_scheduler::{
    LocalHostRuntimeTaskSchedulerHost, DEVICE_ASYNC_STACK_BYTES, DEVICE_BLOCKING_STACK_BYTES,
};

mod task_handle;
#[cfg(unix)]
mod fd_io;
#[cfg(unix)]
pub use fd_io::{scheduleNativeFdReader, NativeFdReader, NativeFdWriter};
pub use task_handle::{
    scheduleNativeBlockingTask, scheduleNativeServiceTask, scheduleNativeServiceTasks,
    NativeTaskHandle,
};
