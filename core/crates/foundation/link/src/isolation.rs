//! Keep an unexpected panic local to one Link operation or stream.
use std::{any::Any, future::{poll_fn, Future}, panic::{catch_unwind, AssertUnwindSafe}, task::Poll};
use crate::CoreLinkError;

fn panicError(context: &str, payload: Box<dyn Any + Send>) -> CoreLinkError {
    let message = payload.downcast_ref::<String>().map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied()).unwrap_or("non-string panic");
    CoreLinkError::new("LINK_OPERATION_PANIC", format!("{context}: {message}"))
}

pub fn sync<T>(context: &str, operation: impl FnOnce() -> Result<T, CoreLinkError>) -> Result<T, CoreLinkError> {
    catch_unwind(AssertUnwindSafe(operation)).map_err(|payload| panicError(context, payload))?
}

/// Poll inside the boundary so panics after an await are isolated as well.
/// Cancellation still drops the original future without a detached worker.
pub async fn future<T>(context: &str, operation: impl Future<Output = Result<T, CoreLinkError>>) -> Result<T, CoreLinkError> {
    let mut operation = std::pin::pin!(operation);
    poll_fn(|cx| match catch_unwind(AssertUnwindSafe(|| operation.as_mut().poll(cx))) {
        Ok(result) => result,
        Err(payload) => Poll::Ready(Err(panicError(context, payload))),
    }).await
}

pub(crate) fn polling<T>(context: &str, operation: impl FnOnce() -> T) -> Result<T, CoreLinkError> {
    catch_unwind(AssertUnwindSafe(operation)).map_err(|payload| panicError(context, payload))
}
