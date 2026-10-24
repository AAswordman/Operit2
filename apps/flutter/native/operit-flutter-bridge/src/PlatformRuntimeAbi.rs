//! ABI module selection is kept outside the platform construction pipeline.
//! Native Core transport uses ABI v3; lifecycle and browser exports have separate owners.

#[cfg(not(target_arch = "wasm32"))]
#[path = "NativeDataRescue.rs"]
pub(crate) mod NativeDataRescue;

#[cfg(target_os = "android")]
#[path = "AndroidJni.rs"]
pub(crate) mod AndroidJni;
#[cfg(not(target_arch = "wasm32"))]
#[path = "FfiTransport.rs"]
pub(crate) mod FfiTransport;
#[cfg(not(target_arch = "wasm32"))]
#[path = "RuntimeBootstrapStore.rs"]
pub(crate) mod RuntimeBootstrapStore;
