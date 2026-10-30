//! mDNS endpoints are dial hints; only the existing pairing secret can authorize an address change.
use super::*;
use operit_host_api::ServiceDiscovery::{DiscoveredService, DiscoverySubscription};

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
