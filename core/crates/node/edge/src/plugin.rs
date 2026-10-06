//! Device-owned implementations of the shared native plugin contract.
use operit_link::CoreValue;
pub use operit_edge_contract::{EdgePluginManifest, EdgePluginCall};
use crate::EdgeServiceError;

/// Firmware registers explicitly adapted actions; arbitrary ToolPkg UI and JS
/// remain owned by Core and never execute on a constrained Edge.
#[async_trait::async_trait(?Send)]
pub trait EdgePlugin: Send + Sync {
    fn manifest(&self) -> EdgePluginManifest;
    fn invoke(&self, action: &str, args: CoreValue) -> Result<CoreValue, EdgeServiceError>;
    /// Wait without blocking the shared UART/runtime task (e.g. main-thread UI).
    async fn invokeAsync(&self, action: &str, args: CoreValue) -> Result<CoreValue, EdgeServiceError> {
        self.invoke(action, args)
    }
}
