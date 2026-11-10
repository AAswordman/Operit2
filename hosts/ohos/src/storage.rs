use std::path::PathBuf;

use operit_host_api::{HostResult};
use operit_host_native_common::NativeRuntimeStorageHost;

#[derive(Clone, Debug)]
pub struct OhosRuntimeStorageHost {
    inner: NativeRuntimeStorageHost,
}

operit_host_api::delegate_runtime_storage!(OhosRuntimeStorageHost, inner);

impl OhosRuntimeStorageHost {
    /// Creates the OpenHarmony runtime storage host with app-owned roots.
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            inner: NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot),
        }
    }
}
