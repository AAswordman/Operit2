//! Recover at the individual operation boundary. No persistent "failed" flag:
//! the next request, peer or sync pass gets its own attempt.
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};

use futures_util::FutureExt;
use operit_link::{CoreCallResponse, CoreLinkError};

fn panicMessage(context: &str, payload: Box<dyn std::any::Any + Send>) -> String {
    let detail = payload.downcast_ref::<String>().map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied()).unwrap_or("non-string panic");
    format!("{context}: {detail}")
}

pub(crate) fn run<T>(context: &str, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(operation))
        .map_err(|payload| panicMessage(context, payload))?
        .map_err(|error| format!("{context}: {error}"))
}

pub(crate) async fn link<T>(context: &str, operation: impl Future<Output = Result<T, CoreLinkError>>) -> Result<T, CoreLinkError> {
    AssertUnwindSafe(operation).catch_unwind().await
        .map_err(|payload| CoreLinkError::new("SPACE_OPERATION_PANIC", panicMessage(context, payload)))?
}

pub(crate) async fn operation<T>(context: &str, operation: impl Future<Output = Result<T, String>>) -> Result<T, String> {
    AssertUnwindSafe(operation).catch_unwind().await
        .map_err(|payload| panicMessage(context, payload))?
        .map_err(|error| format!("{context}: {error}"))
}

pub(crate) async fn call(context: &str, requestId: operit_link::CoreRequestId, operation: impl Future<Output = CoreCallResponse>) -> CoreCallResponse {
    match link(context, async { Ok(operation.await) }).await {
        Ok(response) => response,
        Err(error) => CoreCallResponse::err(requestId, error),
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn a_panicking_operation_does_not_disable_the_next_attempt() {
        let error = super::operation("peer broken snapshot", async { panic!("bad record"); #[allow(unreachable_code)] Ok::<(), String>(()) }).await.unwrap_err();
        assert!(error.contains("peer broken snapshot"));
        assert!(error.contains("bad record"));
        assert_eq!(super::operation("peer healthy snapshot", async { Ok::<_, String>(42) }).await.unwrap(), 42);
        assert_eq!(super::run("next local operation", || Ok(7)).unwrap(), 7);
    }
}
