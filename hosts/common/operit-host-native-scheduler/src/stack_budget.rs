//! Guarded desktop stacks with an exact usable budget for device execution.
#[cfg(windows)]
#[path = "stack_budget_windows.rs"]
mod platform;
#[cfg(unix)]
use corosensei::stack::DefaultStack as BoundedStack;
#[cfg(windows)]
use platform::BoundedStack;

/// Runs a complete device task on a guarded stack, independent of native thread minima.
pub fn withStackBudget<R>(bytes: usize, task: impl FnOnce() -> R) -> std::io::Result<R> {
    let stack = BoundedStack::new(bytes)?;
    Ok(corosensei::on_stack(stack, task))
}
