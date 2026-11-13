//! mDNS endpoints are dial hints; only the existing pairing secret can authorize an address change.
use super::*;
use operit_host_api::ServiceDiscovery::{DiscoveredService, DiscoverySubscription};

pub(super) struct DiscoveryRefresh {
    pub subscription: Option<Box<dyn DiscoverySubscription>>,
    pub scan: Result<(), CoreLinkError>,
}

impl HostRuntimePeerService {
    /// Retains current service endpoints for both paired and unpaired nodes.
    pub(super) fn rememberDiscoveryRecords(&self, records: &[DiscoveredService]) {
        let mut known = self.state.discoveryRecords.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        for record in records {
            known.insert(record.fullName.clone(), record.clone());
        }
    }

    /// Keeps automatic reconnection informed even when the add-device dialog is closed.
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

    /// Actively refreshes the shared Host browser when passive announcements were missed.
    /// Blocking discovery and subscription work stays off the connection coordinator.
    pub(super) async fn refreshReconnectionDiscovery(&self, retrySubscription: bool) -> Result<DiscoveryRefresh, CoreLinkError> {
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
            let _ = sender.send(host.discover("_operit-link._tcp.local.", 1_000));
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

    /// Tries fresh discovered addresses before the saved endpoint, preserving non-network carriers.
    pub(super) fn reconnectionEndpoints(&self, node: &str, record: &StoredOutbound) -> Result<Vec<(String, PeerTransport)>, CoreLinkError> {
        let savedTransport = parseTransport(&record.transport)?;
        let mut endpoints = Vec::new();
        if isNetworkTransport(savedTransport) {
            let records = self.state.discoveryRecords.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
                .values().filter(|record| record.properties.get("nodeId").is_some_and(|id| id == node))
                .cloned().collect();
            for peer in discoveredEndpoints(records, &self.state.nodeId, &BTreeSet::new()) {
                let transport = if peer.address.starts_with("http://") { PeerTransport::Http }
                    else if peer.address.starts_with("ws://") { PeerTransport::WebSocket }
                    else { PeerTransport::Tcp };
                endpoints.push((peer.address, transport));
            }
        }
        let saved = (record.endpoint.clone(), savedTransport);
        if !endpoints.contains(&saved) { endpoints.push(saved); }
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
