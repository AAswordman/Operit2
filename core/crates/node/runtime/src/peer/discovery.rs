//! Discovery sources share one cache and refresh path. Records are untrusted dial hints.
use super::*;
use crate::PeerStateStore::PeerDiscoveryConfig;
use operit_host_api::ServiceDiscovery::{DiscoveredService, DiscoverySubscription};

#[derive(Default)]
pub(super) struct DiscoveryState {
    pub(super) records: Mutex<BTreeMap<String, CachedService>>,
    pub(super) directory: directory::Directory,
}
pub(super) struct CachedService {
    records: Vec<DiscoveredService>,
    pub(super) expires: Option<u64>,
}
pub(super) struct DiscoveryRefresh {
    pub subscription: Option<Box<dyn DiscoverySubscription>>,
    pub scan: Result<(), CoreLinkError>,
}
impl HostRuntimePeerService {
    pub(super) fn discoveryNow(&self) -> Result<u64, CoreLinkError> {
        self.state
            .host
            .hostRuntimeTaskSchedulerHost
            .as_ref()
            .ok_or_else(|| error("Host scheduler is not installed"))?
            .monotonicTimeMillis()
            .map_err(|failure| error(failure.to_string()))
    }
    pub(super) fn directoryConfig(&self) -> Result<Option<PeerDiscoveryConfig>, CoreLinkError> {
        let config = self
            .state
            .store
            .hostConfig()
            .map_err(error)?
            .and_then(|config| config.discovery);
        if self.state.discovery.directory.configure(&config) {
            self.state
                .discovery
                .records
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .retain(|_, entry| entry.expires.is_none());
        }
        Ok(config)
    }
    pub(super) fn needsDirectoryRefresh(&self) -> bool {
        self.directoryConfig()
            .is_ok_and(|config| config.is_some_and(|config| !config.directoryUrl.is_empty()))
    }
    pub(super) fn rememberDiscoveryRecords(&self, records: &[DiscoveredService]) {
        let mut grouped = BTreeMap::<String, Vec<DiscoveredService>>::new();
        for record in records {
            let node = record.properties.get("nodeId").map(String::as_str).unwrap_or("");
            grouped.entry(format!("lan:{}:{node}", record.fullName)).or_default().push(record.clone());
        }
        let mut known = self.state.discovery.records.lock().unwrap_or_else(|p| p.into_inner());
        for (key, records) in grouped { known.insert(key, CachedService { records, expires: None }); }
    }
    pub(super) fn rememberDirectoryRecords(&self, records: &[directory::Announcement]) -> Result<(), CoreLinkError> {
        let now = self.discoveryNow()?;
        let mut known = self.state.discovery.records.lock().unwrap_or_else(|p| p.into_inner());
        known.retain(|_, entry| entry.expires.is_none_or(|expires| expires > now));
        let mut count = known.values().filter(|entry| entry.expires.is_some()).count();
        for announcement in records {
            let services = announcement.records();
            let Some(record) = services.first() else {
                continue;
            };
            if record.properties.get("nodeId") == Some(&self.state.nodeId) {
                continue;
            }
            let key = format!("directory:{}", record.properties["nodeId"]);
            if !known.contains_key(&key) {
                if count >= directory::MAX_PEERS {
                    continue;
                }
                count += 1;
            }
            known.insert(
                key,
                CachedService {
                    records: services,
                    expires: Some(now.saturating_add(announcement.ttlMs.min(directory::LEASE_MS))),
                },
            );
        }
        Ok(())
    }
    pub(super) fn discoveryRecords(&self) -> Vec<DiscoveredService> {
        let _ = self.directoryConfig();
        let now = self.discoveryNow().ok();
        let mut known = self.state.discovery.records.lock().unwrap_or_else(|p| p.into_inner());
        known.retain(|_, entry| entry.expires.is_none_or(|expires| now.is_some_and(|now| expires > now)));
        known.values().flat_map(|entry| entry.records.iter().cloned()).collect()
    }
    pub(super) fn clearDiscovery(&self) {
        self.state
            .discovery
            .records
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }
    pub(super) fn subscribeDiscovery(&self) -> Option<Box<dyn DiscoverySubscription>> {
        let host = self.state.host.serviceDiscoveryHost.as_ref()?;
        let state = Arc::downgrade(&self.state);
        match host.subscribe("_operit-link._tcp.local.", Arc::new(move |record| {
            if let Some(state) = state.upgrade() {
                HostRuntimePeerService { state }.rememberDiscoveryRecords(&[record]);
            }
        })) {
            Ok(subscription) => Some(subscription),
            Err(error) => {
                operit_util::AppLogger::AppLogger::w("RuntimePeerService", &format!("Automatic LAN discovery unavailable: {error}"));
                None
            }
        }
    }

    /// Interactive and background callers use identical source and partial-failure semantics.
    pub(super) async fn refreshDiscovery(&self, timeoutMs: u64, retrySubscription: bool) -> DiscoveryRefresh {
        let (lan, directory) = tokio::join!(
            self.refreshLanDiscovery(timeoutMs, retrySubscription),
            self.refreshDirectory(timeoutMs)
        );
        let mut refresh = lan.unwrap_or_else(|failure| DiscoveryRefresh {
            subscription: None,
            scan: Err(failure),
        });
        if let Err(failure) = &refresh.scan {
            operit_util::AppLogger::AppLogger::w("RuntimePeerService", &format!("LAN discovery unavailable: {failure}"));
        }
        match directory {
            Ok(true) => refresh.scan = Ok(()),
            Ok(false) => {}
            Err(failure) => {
                operit_util::AppLogger::AppLogger::w(
                    "RuntimePeerService",
                    &format!("Directory discovery failed: {failure}"),
                );
                if refresh.scan.is_err() {
                    refresh.scan = Err(failure);
                }
            }
        }
        // Unexpired hints remain useful through a temporary directory outage.
        if refresh.scan.is_err() {
            // Prune leases before deciding whether a failed query can use cached directory hints.
            self.discoveryRecords();
            if self
                .state
                .discovery
                .records
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .values()
                .any(|entry| entry.expires.is_some())
            {
                refresh.scan = Ok(());
            }
        }
        refresh
    }
    async fn refreshLanDiscovery(&self, timeoutMs: u64, retrySubscription: bool) -> Result<DiscoveryRefresh, CoreLinkError> {
        let host = self.state.host.serviceDiscoveryHost.clone()
            .ok_or_else(|| error("Discovery Host is not installed"))?;
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.as_ref()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let service = self.clone();
        let (subscribed, subscription) = tokio::sync::oneshot::channel();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        scheduler.scheduleHostRuntimeTask("peer-rediscovery", Box::new(move || {
            if subscribed.is_closed() { return; }
            let subscription = if retrySubscription { service.subscribeDiscovery() } else { None };
            if subscribed.send(subscription).is_err() || sender.is_closed() { return; }
            let _ = sender.send(host.discover("_operit-link._tcp.local.", timeoutMs));
        })).map_err(|failure| error(failure.to_string()))?;
        // The async worker owns a retried subscription while the blocking scan runs,
        // so stopping it releases the subscription without waiting for discovery.
        let subscription = subscription.await.map_err(|_| error("Background subscription cancelled"))?;
        let records = receiver.await.map_err(|_| error("Background discovery cancelled"))?;
        let scan = records.map_err(|failure| error(failure.to_string())).map(|records| {
            self.rememberDiscoveryRecords(&records);
            operit_util::AppLogger::AppLogger::trace_detail("RuntimePeerService", &format!(
                "Background LAN discovery refreshed records={}", records.len()));
        });
        Ok(DiscoveryRefresh { subscription, scan })
    }
    pub(super) fn discoveredPeerEndpoints(&self, node: &str) -> Vec<(String, PeerTransport)> {
        discoveredEndpoints(self.discoveryRecords(), &self.state.nodeId, &BTreeSet::new())
            .into_iter()
            .filter(|peer| peer.nodeId == node)
            .map(|peer| {
                let mode = if peer.address.starts_with("http://") {
                    PeerTransport::Http
                } else if peer.address.starts_with("ws://") {
                    PeerTransport::WebSocket
                } else {
                    PeerTransport::Tcp
                };
                (peer.address, mode)
            })
            .collect()
    }
    pub(super) fn reconnectionEndpoints(
        &self,
        node: &str,
        record: &StoredOutbound,
    ) -> Result<Vec<(String, PeerTransport)>, CoreLinkError> {
        let transport = parseTransport(&record.transport)?;
        let mut endpoints = if isNetworkTransport(transport) {
            self.discoveredPeerEndpoints(node)
        } else {
            Vec::new()
        };
        let saved = (record.endpoint.clone(), transport);
        if !endpoints.contains(&saved) {
            endpoints.push(saved);
        }
        Ok(endpoints)
    }
    /// Commits only an authenticated address, without restoring removed or replaced credentials.
    pub(super) fn saveAuthenticatedEndpoint(&self, node: &str, original: &StoredOutbound, address: &str, transport: PeerTransport) -> Result<(), CoreLinkError> {
        let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let current = self.outbound(node)?;
        if current.sessionId != original.sessionId || current.sessionSecret != original.sessionSecret {
            return Err(CoreLinkError::new("PEER_PAIRING_CHANGED", "Pairing changed while authenticating the discovered endpoint"));
        }
        // A concurrent operator edit takes precedence over a connection already in flight.
        if current.endpoint != original.endpoint || current.transport != original.transport { return Ok(()); }
        if current.endpoint == address && current.transport == transportName(transport) { return Ok(()); }
        let endpoint = OperatorEndpoint { rendered: address.into(), transport: Some(transport) };
        self.updateOutboundEndpointLocked(node, &endpoint, None)?;
        drop(_guard);
        operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!(
            "Authenticated pairing endpoint refreshed peer={node} endpoint={address}"));
        self.changed();
        Ok(())
    }
}
