//! Host-owned local service discovery contracts independent of transport libraries.

use crate::HostResult;
use core::net::IpAddr;
use std::{collections::BTreeMap, sync::Arc};

/// Contains one resolved service advertisement without platform-specific handles.
#[derive(Clone, Debug)]
pub struct DiscoveredService {
    pub fullName: String,
    pub hostname: String,
    pub addresses: Vec<IpAddr>,
    pub port: u16,
    pub properties: BTreeMap<String, String>,
}

/// Receives resolved advertisements from the host's shared discovery browser.
pub type DiscoveryCallback = Arc<dyn Fn(DiscoveredService) + Send + Sync>;

/// Keeps one announcement listener registered until its owner releases it.
pub trait DiscoverySubscription: Send + Sync {}

/// Owns service browsers and their operating-system resources.
pub trait ServiceDiscoveryHost: Send + Sync {
    /// Collects a snapshot after listening for the requested discovery interval.
    fn discover(&self, serviceType: &str, timeoutMs: u64) -> HostResult<Vec<DiscoveredService>>;

    /// Registers a listener on the shared browser for one service type.
    fn subscribe(
        &self,
        serviceType: &str,
        callback: DiscoveryCallback,
    ) -> HostResult<Box<dyn DiscoverySubscription>>;
}
