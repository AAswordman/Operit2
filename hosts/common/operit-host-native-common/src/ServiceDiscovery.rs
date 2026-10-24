//! Shares one native mDNS daemon and one event stream per service type.

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use operit_host_api::ServiceDiscovery::{
    DiscoveredService, DiscoveryAdvertisement, DiscoveryCallback, DiscoverySubscription,
    ServiceAdvertisement, ServiceDiscoveryHost,
};
use operit_host_api::{HostError, HostResult};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock, Weak},
    time::Duration,
};

#[derive(Default)]
struct BrowserState {
    records: BTreeMap<String, DiscoveredService>,
    callbacks: BTreeMap<u64, DiscoveryCallback>,
    nextListenerId: u64,
    ended: bool,
}

/// Detaches its callback without stopping the shared service browser.
struct ListenerSubscription {
    state: Weak<Mutex<BrowserState>>,
    listenerId: u64,
}

impl DiscoverySubscription for ListenerSubscription {}

impl Drop for ListenerSubscription {
    /// Removes the listener when the owning runtime stops or is dropped.
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state
                .lock()
                .expect("discovery browser state poisoned")
                .callbacks
                .remove(&self.listenerId);
        }
    }
}

/// Owns native discovery resources shared by Core and Edge callers.
#[derive(Default)]
pub struct ServiceDiscoveryProvider {
    daemon: OnceLock<Result<ServiceDaemon, String>>,
    browsers: Mutex<BTreeMap<String, Arc<Mutex<BrowserState>>>>,
}

/// Copies a transport record into the platform-independent host contract.
fn record(info: &ServiceInfo) -> DiscoveredService {
    DiscoveredService {
        fullName: info.get_fullname().to_owned(),
        hostname: info.get_hostname().to_owned(),
        addresses: info.get_addresses().iter().copied().collect(),
        port: info.get_port(),
        properties: info
            .get_properties()
            .iter()
            .map(|property| (property.key().to_owned(), property.val_str().to_owned()))
            .collect(),
    }
}

impl ServiceDiscoveryProvider {
    /// Gets the unique browser for a service type, registering its event loop once.
    fn browser(&self, serviceType: &str) -> HostResult<Arc<Mutex<BrowserState>>> {
        let mut browsers = self
            .browsers
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        if let Some(browser) = browsers.get(serviceType) {
            return Ok(browser.clone());
        }
        let daemon = self
            .daemon
            .get_or_init(|| ServiceDaemon::new().map_err(|error| error.to_string()))
            .as_ref()
            .map_err(|error| HostError::new(error.clone()))?;
        let receiver = daemon
            .browse(serviceType)
            .map_err(|error| HostError::new(error.to_string()))?;
        let browser = Arc::new(Mutex::new(BrowserState::default()));
        let state = browser.clone();
        let scheduled = crate::scheduleNativeServiceTask("operit-service-discovery", move || {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let resolved = record(&info);
                        let callbacks = {
                            let mut state = state.lock().expect("discovery browser state poisoned");
                            state
                                .records
                                .insert(resolved.fullName.clone(), resolved.clone());
                            state.callbacks.values().cloned().collect::<Vec<_>>()
                        };
                        for callback in callbacks {
                            callback(resolved.clone());
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullName) => {
                        state
                            .lock()
                            .expect("discovery browser state poisoned")
                            .records
                            .remove(&fullName);
                    }
                    _ => {}
                }
            }
            state
                .lock()
                .expect("discovery browser state poisoned")
                .ended = true;
        });
        if let Err(error) = scheduled {
            daemon.stop_browse(serviceType).map_err(|cleanup| {
                HostError::new(format!("{error}; stop rejected service browser: {cleanup}"))
            })?;
            return Err(error);
        }
        browsers.insert(serviceType.to_owned(), browser.clone());
        Ok(browser)
    }
}

struct Advertisement {
    daemon: ServiceDaemon,
    name: String,
}
impl DiscoveryAdvertisement for Advertisement {}
impl Drop for Advertisement {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.name);
    }
}
impl ServiceDiscoveryHost for ServiceDiscoveryProvider {
    /// Declares the multicast advertisements implemented by this provider.
    fn supportsAdvertisement(&self) -> bool {
        true
    }

    /// Publishes a Host-owned LAN service advertisement.
    fn advertise(
        &self,
        service: ServiceAdvertisement,
    ) -> HostResult<Box<dyn DiscoveryAdvertisement>> {
        let daemon = self
            .daemon
            .get_or_init(|| ServiceDaemon::new().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| HostError::new(e.clone()))?
            .clone();
        let info = ServiceInfo::new(
            &service.serviceType,
            &service.instance,
            &service.hostname,
            "",
            service.port,
            service
                .properties
                .into_iter()
                .collect::<std::collections::HashMap<_, _>>(),
        )
        .map_err(|e| HostError::new(e.to_string()))?
        .enable_addr_auto();
        let name = info.get_fullname().to_string();
        daemon
            .register(info)
            .map_err(|e| HostError::new(e.to_string()))?;
        Ok(Box::new(Advertisement { daemon, name }))
    }
    /// Collects resolved records without creating competing multicast consumers.
    fn discover(&self, serviceType: &str, timeoutMs: u64) -> HostResult<Vec<DiscoveredService>> {
        if timeoutMs == 0 {
            return Err(HostError::new("service discovery timeout must be positive"));
        }
        let browser = self.browser(serviceType)?;
        std::thread::sleep(Duration::from_millis(timeoutMs));
        let state = browser
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        if state.ended {
            return Err(HostError::new("service discovery browser has stopped"));
        }
        Ok(state.records.values().cloned().collect())
    }

    /// Subscribes to the same browser used by interactive scans.
    fn subscribe(
        &self,
        serviceType: &str,
        callback: DiscoveryCallback,
    ) -> HostResult<Box<dyn DiscoverySubscription>> {
        let browser = self.browser(serviceType)?;
        let (records, listenerId) = {
            let mut state = browser
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            if state.ended {
                return Err(HostError::new("service discovery browser has stopped"));
            }
            let listenerId = state.nextListenerId;
            state.nextListenerId = listenerId
                .checked_add(1)
                .ok_or_else(|| HostError::new("service discovery listener id overflow"))?;
            state.callbacks.insert(listenerId, callback.clone());
            (
                state.records.values().cloned().collect::<Vec<_>>(),
                listenerId,
            )
        };
        for record in records {
            callback(record);
        }
        Ok(Box::new(ListenerSubscription {
            state: Arc::downgrade(&browser),
            listenerId,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies host records retain protocol TXT data without exposing mDNS handles.
    #[test]
    fn copiesResolvedServiceProperties() {
        let info = ServiceInfo::new(
            "_operit-edge._tcp.local.",
            "test-edge",
            "edge.local.",
            "192.168.8.11",
            37195,
            std::collections::HashMap::from([("deviceId".to_owned(), "edge-1".to_owned())]),
        )
        .unwrap();
        let resolved = record(&info);
        assert_eq!(resolved.port, 37195);
        assert_eq!(resolved.properties.get("deviceId").unwrap(), "edge-1");
        assert_eq!(
            resolved.addresses,
            vec!["192.168.8.11".parse::<std::net::IpAddr>().unwrap()]
        );
    }

    /// Verifies zero-duration scans fail before allocating a native daemon.
    #[test]
    fn rejectsZeroDurationWithoutStartingDaemon() {
        let host = ServiceDiscoveryProvider::default();
        assert!(host.discover("_operit._tcp.local.", 0).is_err());
        assert!(host.daemon.get().is_none());
    }
    /// Verifies dropping a listener releases its callback without affecting records.
    #[test]
    fn subscriptionDropReleasesCallback() {
        let state = Arc::new(Mutex::new(BrowserState::default()));
        state.lock().unwrap().callbacks.insert(0, Arc::new(|_| {}));
        let subscription = ListenerSubscription {
            state: Arc::downgrade(&state),
            listenerId: 0,
        };
        drop(subscription);
        assert!(state.lock().unwrap().callbacks.is_empty());
    }
}
