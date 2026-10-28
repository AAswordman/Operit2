//! Owns Android Wi-Fi multicast access at the Host discovery resource boundary.

use crate::secret_store::{androidHostSecretStoreBridge, AndroidHostSecretStoreBridge};
use jni::objects::GlobalRef;
use operit_host_api::{HostError, HostResult};
use std::sync::Arc;

struct MulticastLease {
    bridge: Arc<AndroidHostSecretStoreBridge>,
    lock: Option<GlobalRef>,
}

/// Acquires Wi-Fi multicast access before the shared daemon creates its sockets.
pub(crate) fn acquireMulticastLease() -> HostResult<Box<dyn Send + Sync>> {
    let bridge = androidHostSecretStoreBridge()?;
    let lock = {
        let mut env = crate::attachCurrentThread(&bridge.vm).map_err(hostError)?;
        let lock = env
            .call_method(
                bridge.host.as_obj(),
                "acquireDiscoveryMulticastLock",
                "()Landroid/net/wifi/WifiManager$MulticastLock;",
                &[],
            )
            .map_err(hostError)?
            .l()
            .map_err(hostError)?;
        // Ethernet-only Android hosts do not own a Wi-Fi multicast filter.
        if lock.is_null() {
            None
        } else {
            match env.new_global_ref(&lock) {
                Ok(owned) => Some(owned),
                Err(error) => {
                    env.call_method(&lock, "release", "()V", &[])
                        .map_err(hostError)?;
                    return Err(hostError(error));
                }
            }
        }
    };
    Ok(Box::new(MulticastLease { bridge, lock }))
}

/// Adds the discovery capability context to JNI failures.
fn hostError(error: impl std::fmt::Display) -> HostError {
    HostError::new(format!(
        "Android discovery multicast access failed: {error}"
    ))
}

impl Drop for MulticastLease {
    /// Releases the exact Wi-Fi lock after its daemon has closed the multicast sockets.
    fn drop(&mut self) {
        if let Some(lock) = &self.lock {
            let result = crate::attachCurrentThread(&self.bridge.vm)
                .map_err(hostError)
                .and_then(|mut env| {
                    env.call_method(lock.as_obj(), "release", "()V", &[])
                        .map(|_| ())
                        .map_err(hostError)
                });
            if let Err(error) = result {
                operit_host_api::tryLogHostConsole(6, "ServiceDiscovery", &error.to_string());
            }
        }
    }
}
