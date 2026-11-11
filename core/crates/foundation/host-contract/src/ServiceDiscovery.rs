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
/// 删除/停止监听时释放广播，与浏览器订阅分开拥有资源。
pub trait DiscoveryAdvertisement: Send + Sync {}
#[derive(Clone, Debug)]
pub struct ServiceAdvertisement {
    pub serviceType: String, pub instance: String, pub hostname: String,
    pub port: u16, pub properties: BTreeMap<String, String>,
}

/// Owns service browsers and their operating-system resources.
pub trait ServiceDiscoveryHost: Send + Sync {
    /// Enumerates this Host's interface addresses without browsing or advertising.
    /// QR connections must work even when multicast discovery is disabled.
    fn localAddresses(&self) -> HostResult<Vec<IpAddr>> {
        Err(crate::HostError::new("This Host cannot enumerate local network addresses"))
    }

    /// Declares advertisement support independently of browsing discovered services.
    fn supportsAdvertisement(&self) -> bool;
    fn advertise(&self, _service: ServiceAdvertisement) -> HostResult<Box<dyn DiscoveryAdvertisement>> {
        Err(crate::HostError::new("Service advertisement is not supported by this Host"))
    }
    /// Collects a snapshot after listening for the requested discovery interval.
    fn discover(&self, serviceType: &str, timeoutMs: u64) -> HostResult<Vec<DiscoveredService>>;

    /// Registers a listener on the shared browser for one service type.
    fn subscribe(
        &self,
        serviceType: &str,
        callback: DiscoveryCallback,
    ) -> HostResult<Box<dyn DiscoverySubscription>>;
}
