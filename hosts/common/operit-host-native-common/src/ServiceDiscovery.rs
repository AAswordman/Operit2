//! Shares one mDNS daemon and one actively refreshable browser per service type.

use mdns_sd::{IfKind, DaemonEvent, ServiceDaemon, ServiceEvent, ServiceInfo};
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

/// Acquires the embedding Host's network resource for the daemon's entire lifetime.
pub type NetworkLeaseFactory = fn() -> HostResult<Box<dyn Send + Sync>>;

#[derive(Default)]
struct BrowserState {
    records: BTreeMap<String, DiscoveredService>,
    callbacks: BTreeMap<u64, DiscoveryCallback>,
    nextListenerId: u64,
    search: Option<String>,
    ended: bool,
}

impl BrowserState {
    /// Rejects snapshots from a terminated browser instead of reporting no devices.
    fn snapshot(&self) -> HostResult<Vec<DiscoveredService>> {
        if self.ended {
            return Err(HostError::new("service discovery browser has stopped"));
        }
        Ok(self.records.values().cloned().collect())
    }
}

struct Browser {
    state: Arc<Mutex<BrowserState>>,
    refresh: flume::Sender<flume::Sender<HostResult<()>>>,
}

impl Browser {
    /// Requests a new query on the shared browser and waits for its admission.
    fn refresh(&self) -> HostResult<()> {
        let (reply, result) = flume::bounded(1);
        self.refresh.send(reply).map_err(hostError)?;
        result.recv().map_err(hostError)?
    }
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

struct Daemon {
    service: ServiceDaemon,
    error: Arc<Mutex<Option<String>>>,
    _networkLease: Option<Box<dyn Send + Sync>>,
}

impl Daemon {
    /// Acquires platform resources before opening sockets and monitors daemon errors.
    fn new(networkLease: Option<NetworkLeaseFactory>, interface: Option<std::net::IpAddr>) -> HostResult<Self> {
        let lease = networkLease.map(|acquire| acquire()).transpose()?;
        let service = ServiceDaemon::new().map_err(hostError)?;
        if let Some(address) = interface {
            service.disable_interface(IfKind::All).map_err(hostError)?;
            service.enable_interface(IfKind::Addr(address)).map_err(hostError)?;
        }
        let monitor = match service.monitor() {
            Ok(monitor) => monitor,
            Err(error) => {
                service
                    .shutdown()
                    .map_err(|cleanup| hostError(format!("{error}; shutdown daemon: {cleanup}")))?;
                return Err(hostError(error));
            }
        };
        let error = Arc::new(Mutex::new(None));
        let state = error.clone();
        if let Err(error) =
            crate::scheduleNativeServiceTask("operit-discovery-monitor", move || {
                while let Ok(event) = monitor.recv() {
                    let priority = match &event {
                        DaemonEvent::Error(error) => {
                            *state.lock().expect("discovery daemon state poisoned") =
                                Some(error.to_string());
                            6
                        }
                        _ => 4,
                    };
                    operit_host_api::tryLogHostConsole(
                        priority,
                        "ServiceDiscovery",
                        &format!("{event:?}"),
                    );
                }
            })
        {
            service
                .shutdown()
                .map_err(|cleanup| hostError(format!("{error}; shutdown daemon: {cleanup}")))?;
            return Err(error);
        }
        Ok(Self {
            service,
            error,
            _networkLease: lease,
        })
    }

    /// Surfaces asynchronous daemon failures to interactive discovery callers.
    fn check(&self) -> HostResult<()> {
        match self.error.lock().map_err(hostError)?.as_ref() {
            Some(error) => Err(hostError(format!(
                "service discovery daemon failed: {error}"
            ))),
            None => Ok(()),
        }
    }
}

impl Drop for Daemon {
    /// Stops sockets before releasing the embedding Host's network lease.
    fn drop(&mut self) {
        let result = self
            .service
            .shutdown()
            .map_err(hostError)
            .and_then(|done| done.recv_timeout(Duration::from_secs(2)).map_err(hostError));
        if let Err(error) = result {
            operit_host_api::tryLogHostConsole(
                6,
                "ServiceDiscovery",
                &format!("daemon shutdown failed: {error}"),
            );
        }
    }
}

/// Owns native discovery resources shared by Core and Edge callers.
#[derive(Default)]
pub struct ServiceDiscoveryProvider {
    interface: Option<std::net::IpAddr>,
    daemon: OnceLock<Result<Daemon, String>>,
    browsers: Mutex<BTreeMap<String, Arc<Browser>>>,
    networkLease: Option<NetworkLeaseFactory>,
}

/// Converts transport and synchronization failures into the Host error contract.
fn hostError(error: impl std::fmt::Display) -> HostError {
    HostError::new(error.to_string())
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

/// Applies typed browser events while invoking subscribers outside the state lock.
fn applyEvent(state: &Mutex<BrowserState>, event: ServiceEvent) {
    match event {
        ServiceEvent::SearchStarted(search) => {
            operit_host_api::tryLogHostConsole(4, "ServiceDiscovery", &search);
            state
                .lock()
                .expect("discovery browser state poisoned")
                .search = Some(search);
        }
        ServiceEvent::ServiceResolved(info) => {
            let resolved = record(&info);
            operit_host_api::tryLogHostConsole(
                4,
                "ServiceDiscovery",
                &format!(
                    "resolved service={} addresses={:?} port={}",
                    resolved.fullName, resolved.addresses, resolved.port
                ),
            );
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
        ServiceEvent::SearchStopped(_) => {
            state
                .lock()
                .expect("discovery browser state poisoned")
                .ended = true;
        }
        _ => {}
    }
}

enum BrowserInput {
    Refresh(Result<flume::Sender<HostResult<()>>, flume::RecvError>),
    Event(Result<ServiceEvent, flume::RecvError>),
}

/// Serializes query restarts without replacing shared state or subscription handles.
fn runBrowser(
    daemon: ServiceDaemon,
    serviceType: String,
    state: Arc<Mutex<BrowserState>>,
    mut receiver: flume::Receiver<ServiceEvent>,
    refresh: flume::Receiver<flume::Sender<HostResult<()>>>,
) {
    loop {
        let input = flume::Selector::new()
            .recv(&refresh, BrowserInput::Refresh)
            .recv(&receiver, BrowserInput::Event)
            .wait();
        match input {
            BrowserInput::Refresh(Ok(reply)) => {
                {
                    let mut state = state.lock().expect("discovery browser state poisoned");
                    state.records.clear();
                    state.search = None;
                }
                // Remove the old exponential query timer before issuing a fresh browse.
                let result = daemon
                    .stop_browse(&serviceType)
                    .and_then(|_| daemon.browse(&serviceType))
                    .map(|next| {
                        receiver = next;
                    })
                    .map_err(hostError);
                let failed = result.is_err();
                let _ = reply.send(result);
                if failed {
                    break;
                }
            }
            BrowserInput::Event(Ok(event)) => applyEvent(&state, event),
            BrowserInput::Refresh(Err(_)) | BrowserInput::Event(Err(_)) => break,
        }
    }
    state
        .lock()
        .expect("discovery browser state poisoned")
        .ended = true;
    if let Err(error) = daemon.stop_browse(&serviceType) {
        operit_host_api::tryLogHostConsole(
            6,
            "ServiceDiscovery",
            &format!("stop browser {serviceType}: {error}"),
        );
    }
}

impl ServiceDiscoveryProvider {
    /// Restricts discovery to the interface selected by a device embedding.
    pub fn forInterface(address: std::net::IpAddr) -> Self {
        Self { interface: Some(address), ..Self::default() }
    }

    /// Installs the embedding Host's multicast resource acquisition policy.
    pub fn withNetworkLease(acquire: NetworkLeaseFactory) -> Self {
        Self {
            networkLease: Some(acquire),
            ..Self::default()
        }
    }

    /// Returns the shared daemon or the explicit initialization failure.
    fn daemon(&self) -> HostResult<&Daemon> {
        let daemon = self
            .daemon
            .get_or_init(|| Daemon::new(self.networkLease, self.interface).map_err(|error| error.to_string()))
            .as_ref()
            .map_err(hostError)?;
        daemon.check()?;
        Ok(daemon)
    }

    /// Gets the unique browser and actively refreshes it when a scan is requested.
    fn browser(&self, serviceType: &str, refresh: bool) -> HostResult<Arc<Browser>> {
        let mut browsers = self.browsers.lock().map_err(hostError)?;
        if let Some(browser) = browsers.get(serviceType) {
            browser.state.lock().map_err(hostError)?.snapshot()?;
            if refresh {
                browser.refresh()?;
            }
            return Ok(browser.clone());
        }
        let daemon = self.daemon()?.service.clone();
        let receiver = daemon.browse(serviceType).map_err(hostError)?;
        let (sender, commands) = flume::unbounded();
        let browser = Arc::new(Browser {
            state: Arc::new(Mutex::new(BrowserState::default())),
            refresh: sender,
        });
        let state = browser.state.clone();
        let service = serviceType.to_owned();
        let worker = daemon.clone();
        if let Err(error) =
            crate::scheduleNativeServiceTask("operit-service-discovery", move || {
                runBrowser(worker, service, state, receiver, commands);
            })
        {
            daemon.stop_browse(serviceType).map_err(|cleanup| {
                hostError(format!("{error}; stop rejected browser: {cleanup}"))
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
    /// Withdraws the advertisement when its listener owner releases it.
    fn drop(&mut self) {
        if let Err(error) = self.daemon.unregister(&self.name) {
            operit_host_api::tryLogHostConsole(
                6,
                "ServiceDiscovery",
                &format!("unregister {}: {error}", self.name),
            );
        }
    }
}

impl ServiceDiscoveryHost for ServiceDiscoveryProvider {
    fn localAddresses(&self) -> HostResult<Vec<std::net::IpAddr>> {
        localNetworkAddresses()
    }

    /// Declares the multicast advertisements implemented by this provider.
    fn supportsAdvertisement(&self) -> bool {
        true
    }

    /// Publishes a Host-owned LAN service advertisement.
    fn advertise(
        &self,
        service: ServiceAdvertisement,
    ) -> HostResult<Box<dyn DiscoveryAdvertisement>> {
        let daemon = self.daemon()?.service.clone();
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
        .map_err(hostError)?
        .enable_addr_auto();
        let name = info.get_fullname().to_string();
        daemon.register(info).map_err(hostError)?;
        Ok(Box::new(Advertisement { daemon, name }))
    }

    /// Sends a fresh query and collects resolved records from the shared browser.
    fn discover(&self, serviceType: &str, timeoutMs: u64) -> HostResult<Vec<DiscoveredService>> {
        if timeoutMs == 0 {
            return Err(hostError("service discovery timeout must be positive"));
        }
        let browser = self.browser(serviceType, true)?;
        std::thread::sleep(Duration::from_millis(timeoutMs));
        self.daemon()?.check()?;
        let state = browser.state.lock().map_err(hostError)?;
        let records = state.snapshot()?;
        operit_host_api::tryLogHostConsole(
            4,
            "ServiceDiscovery",
            &format!(
                "scan complete service={serviceType} records={} search={:?}",
                records.len(),
                state.search
            ),
        );
        Ok(records)
    }

    /// Subscribes to the same browser used by interactive scans.
    fn subscribe(
        &self,
        serviceType: &str,
        callback: DiscoveryCallback,
    ) -> HostResult<Box<dyn DiscoverySubscription>> {
        let browser = self.browser(serviceType, false)?;
        let (records, listenerId) = {
            let mut state = browser.state.lock().map_err(hostError)?;
            let records = state.snapshot()?;
            let listenerId = state.nextListenerId;
            state.nextListenerId = listenerId
                .checked_add(1)
                .ok_or_else(|| hostError("service discovery listener id overflow"))?;
            state.callbacks.insert(listenerId, callback.clone());
            (records, listenerId)
        };
        for record in records {
            callback(record);
        }
        Ok(Box::new(ListenerSubscription {
            state: Arc::downgrade(&browser.state),
            listenerId,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constructs a protocol-complete advertisement without network resources.
    fn info() -> ServiceInfo {
        ServiceInfo::new(
            "_operit-link._tcp.local.",
            "test-peer",
            "peer.local.",
            "192.168.8.11",
            37195,
            std::collections::HashMap::from([("nodeId".to_owned(), "peer-1".to_owned())]),
        )
        .unwrap()
    }

    /// Verifies resolved records retain addresses and protocol TXT properties.
    #[test]
    fn copiesResolvedServiceProperties() {
        let resolved = record(&info());
        assert_eq!(resolved.port, 37195);
        assert_eq!(resolved.properties.get("nodeId").unwrap(), "peer-1");
        assert_eq!(
            resolved.addresses,
            vec!["192.168.8.11".parse::<std::net::IpAddr>().unwrap()]
        );
    }

    /// Rejects invalid scan durations before acquiring sockets or platform resources.
    #[test]
    fn rejectsZeroDurationWithoutStartingDaemon() {
        let host = ServiceDiscoveryProvider::default();
        assert!(host.discover("_operit-link._tcp.local.", 0).is_err());
        assert!(host.daemon.get().is_none());
    }

    /// Releases callbacks when their runtime subscription is dropped.
    #[test]
    fn subscriptionDropReleasesCallback() {
        let state = Arc::new(Mutex::new(BrowserState::default()));
        state.lock().unwrap().callbacks.insert(0, Arc::new(|_| {}));
        drop(ListenerSubscription {
            state: Arc::downgrade(&state),
            listenerId: 0,
        });
        assert!(state.lock().unwrap().callbacks.is_empty());
    }

    /// Keeps subscribers alive across searches and invokes them outside the lock.
    #[test]
    fn searchEventsPreserveSubscriptionsAndRecords() {
        let state = Arc::new(Mutex::new(BrowserState::default()));
        let weak = Arc::downgrade(&state);
        state.lock().unwrap().callbacks.insert(
            0,
            Arc::new(move |_| {
                assert!(weak.upgrade().unwrap().try_lock().is_ok());
            }),
        );
        applyEvent(&state, ServiceEvent::ServiceResolved(info()));
        applyEvent(&state, ServiceEvent::SearchStarted("fresh query".into()));
        assert_eq!(state.lock().unwrap().snapshot().unwrap().len(), 1);
        assert_eq!(state.lock().unwrap().callbacks.len(), 1);
        applyEvent(
            &state,
            ServiceEvent::ServiceRemoved(
                "_operit-link._tcp.local.".into(),
                info().get_fullname().to_string(),
            ),
        );
        assert!(state.lock().unwrap().snapshot().unwrap().is_empty());
    }

    /// Reports stopped browsers as errors rather than successful empty scans.
    #[test]
    fn stoppedBrowserRejectsSnapshots() {
        let state = Mutex::new(BrowserState::default());
        applyEvent(
            &state,
            ServiceEvent::SearchStopped("_operit-link._tcp.local.".into()),
        );
        assert!(state.lock().unwrap().snapshot().is_err());
    }

    /// Ensures each refresh is admitted by the existing worker before a scan starts.
    #[test]
    fn refreshWaitsForWorkerAdmission() {
        let (sender, commands) = flume::unbounded::<flume::Sender<HostResult<()>>>();
        let browser = Browser {
            state: Arc::new(Mutex::new(BrowserState::default())),
            refresh: sender,
        };
        let worker = std::thread::spawn(move || {
            for _ in 0..2 {
                commands.recv().unwrap().send(Ok(())).unwrap();
            }
        });
        browser.refresh().unwrap();
        browser.refresh().unwrap();
        worker.join().unwrap();
    }

    /// Verifies real daemon queries restart while the shared browser and callback survive.
    #[test]
    fn refreshIssuesNewQueriesOnTheSameBrowser() {
        let host = ServiceDiscoveryProvider::default();
        let browser = host.browser("_operit-link._tcp.local.", false).unwrap();
        let subscription = host
            .subscribe("_operit-link._tcp.local.", Arc::new(|_| {}))
            .unwrap();
        let metrics = || {
            host.daemon()
                .unwrap()
                .service
                .get_metrics()
                .unwrap()
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
        };
        let before = *metrics().get("browse").unwrap();
        let refreshed = host.browser("_operit-link._tcp.local.", true).unwrap();
        let after = *metrics().get("browse").unwrap();
        assert!(Arc::ptr_eq(&browser, &refreshed));
        assert!(after > before);
        assert_eq!(browser.state.lock().unwrap().callbacks.len(), 1);
        drop(subscription);
        assert!(browser.state.lock().unwrap().callbacks.is_empty());
    }

    /// Exercises real multicast resolution and the advertised TCP listener on a LAN.
    #[test]
    #[ignore = "requires a multicast-enabled LAN interface"]
    fn liveLanAdvertisementResolvesToListeningPort() {
        use std::net::{SocketAddr, TcpListener, TcpStream};
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let publisher = ServiceDiscoveryProvider::default();
        let scanner = ServiceDiscoveryProvider::default();
        let node = format!("operit-discovery-test-{}", std::process::id());
        let advertisement = publisher
            .advertise(ServiceAdvertisement {
                serviceType: "_operit-link._tcp.local.".into(),
                instance: node.clone(),
                hostname: format!("{node}.local."),
                port: listener.local_addr().unwrap().port(),
                properties: [
                    ("nodeId".into(), node.clone()),
                    ("displayName".into(), "LAN discovery test".into()),
                    ("transports".into(), "tcp".into()),
                ]
                .into(),
            })
            .unwrap();
        let records = scanner.discover("_operit-link._tcp.local.", 5000).unwrap();
        let service = records
            .iter()
            .find(|record| record.properties.get("nodeId") == Some(&node))
            .expect("the live advertisement must be discovered");
        let ip = service
            .addresses
            .iter()
            .find(|ip| ip.is_ipv4() && !ip.is_loopback())
            .expect("the LAN advertisement must resolve to a non-loopback IPv4 address");
        let connection =
            TcpStream::connect_timeout(&SocketAddr::new(*ip, service.port), Duration::from_secs(2))
                .unwrap();
        let (accepted, _) = listener.accept().unwrap();
        assert_eq!(
            accepted.local_addr().unwrap().port(),
            listener.local_addr().unwrap().port()
        );
        assert_eq!(connection.peer_addr().unwrap().ip(), *ip);
        drop(advertisement);
    }
}

/// Interface enumeration does not start the discovery daemon or acquire multicast resources.
pub fn localNetworkAddresses() -> HostResult<Vec<std::net::IpAddr>> {
    let mut addresses = if_addrs::get_if_addrs().map_err(hostError)?
        .into_iter().map(|interface| interface.ip()).collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    Ok(addresses)
}
