#![allow(non_snake_case)]

mod runtime_event_scheduler;
mod runtime_task_scheduler;

pub use runtime_event_scheduler::NativeHostRuntimeEventSchedulerHost;
pub use runtime_task_scheduler::NativeHostRuntimeTaskSchedulerHost;
