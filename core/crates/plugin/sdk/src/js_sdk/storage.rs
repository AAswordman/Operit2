//! Typed plugin-owned databases exposed through the existing JavaScript Host.
use super::JsFuture;
use super::results::StorageRequestResultData;
pub use operit_host_api::PluginStorage::*;

/// Dispatches typed requests; database object facades retain only authenticated handles.
pub trait StorageHost: Send + Sync {
    /// Performs one parameterized read or atomic bounded write through the ordinary tool-call pipeline.
    fn request(&self, request: StorageRequest) -> JsFuture<StorageRequestResultData>;
}
