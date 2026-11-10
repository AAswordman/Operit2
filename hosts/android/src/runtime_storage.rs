use std::path::PathBuf;

use operit_host_api::{HostResult, HostSecretStore};

#[derive(Clone, Debug)]
pub struct AndroidRuntimeStorageHost {
    inner: operit_host_native_common::NativeRuntimeStorageHost,
}

operit_host_api::delegate_runtime_storage!(AndroidRuntimeStorageHost, inner);

impl AndroidRuntimeStorageHost {
    /// Creates an Android runtime storage host with explicit runtime and workspace roots.
    #[allow(non_snake_case)]
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            inner: operit_host_native_common::NativeRuntimeStorageHost::new(
                runtimeRoot,
                workspaceRoot,
            ),
        }
    }
}

impl HostSecretStore for AndroidRuntimeStorageHost {
    /// Reads secret bytes from the Android system-backed host secret store.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        #[cfg(target_os = "android")]
        {
            crate::secret_store::androidHostSecretStoreBridge()?.readSecret(key)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = key;
            Err(operit_host_api::HostError::new(
                "Android host secret store is available on Android targets",
            ))
        }
    }

    /// Writes secret bytes into the Android system-backed host secret store.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        #[cfg(target_os = "android")]
        {
            crate::secret_store::androidHostSecretStoreBridge()?.writeSecret(key, content)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (key, content);
            Err(operit_host_api::HostError::new(
                "Android host secret store is available on Android targets",
            ))
        }
    }

    /// Deletes secret bytes from the Android system-backed host secret store.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        #[cfg(target_os = "android")]
        {
            crate::secret_store::androidHostSecretStoreBridge()?.deleteSecret(key)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = key;
            Err(operit_host_api::HostError::new(
                "Android host secret store is available on Android targets",
            ))
        }
    }
}
