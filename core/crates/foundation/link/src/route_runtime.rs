use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use crate::{CoreCallRequest, CoreCallResponse, CoreEventStream, CoreLinkError, CoreWatchRequest};

tokio::task_local! {
    static CORE_FORCE_LOCAL: bool;
}

/// Provides the Rust-internal route gate used by annotation-generated wrappers.
pub trait CoreRouteRuntime: Send + Sync {
    /// Determines whether one annotated invocation targets another CoreNode.
    fn shouldRoute(&self, methodName: &str, args: &crate::CoreValue)
        -> Result<bool, CoreLinkError>;

    /// Determines whether one annotated watch should be owned by the route runtime.
    #[allow(non_snake_case)]
    fn shouldRouteWatch(
        &self,
        methodName: &str,
        args: &crate::CoreValue,
    ) -> Result<bool, CoreLinkError> {
        self.shouldRoute(methodName, args)
    }

    /// Determines whether one annotated watch should begin from the already borrowed local Core source.
    #[allow(non_snake_case)]
    fn shouldUseLocalWatchSource(
        &self,
        _methodName: &str,
        _args: &crate::CoreValue,
    ) -> Result<bool, CoreLinkError> {
        Ok(false)
    }

    /// Routes one annotated asynchronous call through the active CoreNode graph.
    fn call(&self, request: CoreCallRequest) -> Pin<Box<dyn Future<Output = CoreCallResponse>>>;

    /// Routes one annotated StateFlow watch through the active CoreNode graph.
    fn watch(
        &self,
        request: CoreWatchRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>>;

    /// Routes one annotated StateFlow watch while carrying a local source opened by the wrapper.
    #[allow(non_snake_case)]
    fn watchWithLocalSource(
        &self,
        request: CoreWatchRequest,
        _localStream: CoreEventStream,
    ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>> {
        self.watch(request)
    }
}

static CORE_ROUTE_RUNTIME: OnceLock<RwLock<Option<Arc<dyn CoreRouteRuntime>>>> = OnceLock::new();
static CORE_ROUTE_REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Creates a unique request identifier for one annotation wrapper invocation.
pub fn nextCoreRouteRequestId(methodName: &str) -> String {
    let sequence = CORE_ROUTE_REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("core-route-{methodName}-{sequence}")
}

/// Returns the process-local storage slot for the active route runtime.
fn routeRuntimeSlot() -> &'static RwLock<Option<Arc<dyn CoreRouteRuntime>>> {
    CORE_ROUTE_RUNTIME.get_or_init(|| RwLock::new(None))
}

/// Installs the process-local Rust route runtime used by annotated wrappers.
pub fn installCoreRouteRuntime(runtime: Arc<dyn CoreRouteRuntime>) {
    *routeRuntimeSlot()
        .write()
        .expect("Core route runtime lock poisoned") = Some(runtime);
}

/// Removes the active Rust route runtime during Core shutdown.
pub fn clearCoreRouteRuntime() {
    *routeRuntimeSlot()
        .write()
        .expect("Core route runtime lock poisoned") = None;
}

/// Returns the installed route runtime for one generated wrapper invocation.
pub fn coreRouteRuntime() -> Option<Arc<dyn CoreRouteRuntime>> {
    routeRuntimeSlot()
        .read()
        .expect("Core route runtime lock poisoned")
        .clone()
}

/// Executes one local Core operation while suppressing annotation re-routing.
pub async fn withCoreForceLocal<F: Future>(future: F) -> F::Output {
    CORE_FORCE_LOCAL.scope(true, future).await
}

/// Reports whether the current Rust call stack requires local annotation execution.
pub fn coreForceLocal() -> bool {
    CORE_FORCE_LOCAL.try_with(|value| *value).unwrap_or(false)
}
