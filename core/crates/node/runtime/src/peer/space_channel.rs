//! Explicit return-channel grants exchanged over an authenticated pairing after
//! Space admission. These are scoped transport credentials, not reverse pairing
//! and not a role assignment. Every use rechecks current membership/revocation.
use super::*;

const INBOUND: &str = "runtime/link_access/space_channel_inbound.preferences.json";
const OUTBOUND: &str = "runtime/link_access/space_channel_outbound.preferences.json";
pub(super) const TARGET: &str = "$peer.space-channel";

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub(super) struct Grant {
    pub id: String,
    pub spaceId: String,
    pub peer: String,
    pub pairingId: String,
    pub secret: String,
    pub port: u16,
    pub transport: PeerTransport,
    pub endpoint: String,
}

// No role or capability is taken from this wire record. The authenticated
// sender identity and locally replayed admission operations are authoritative.
#[derive(Serialize, Deserialize)]
struct Offer {
    id: String,
    spaceId: String,
    secret: String,
    port: u16,
    transport: PeerTransport,
}

impl HostRuntimePeerService {
    fn spaceChannelScope(&self, peer: &str) -> Result<Option<String>, CoreLinkError> {
        self.router()?.spaceChannelScope(peer)
    }

    pub(super) fn spaceInbound(&self, id: &str, peer: &str) -> Result<Grant, CoreLinkError> {
        let grant = self.state.store.records::<Grant>(INBOUND).map_err(error)?
            .remove(id).ok_or_else(|| error("Space channel credential not found"))?;
        if grant.peer != peer || self.spaceChannelScope(peer)?.as_deref() != Some(&grant.spaceId)
            || !self.state.store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
                .map_err(error)?.get(&grant.pairingId).is_some_and(|r| r.peerNodeId == peer && r.pairingServiceVersion == PAIRING_SERVICE_VERSION) {
            return Err(CoreLinkError::new("SPACE_CHANNEL_REVOKED", "Space membership or original pairing no longer authorizes this channel"));
        }
        Ok(grant)
    }

    pub(super) fn spaceOutbound(&self, peer: &str) -> Result<Grant, CoreLinkError> {
        let scope = self.spaceChannelScope(peer)?;
        let credentials = self.inboundCredentials()?;
        self.state.store.records::<Grant>(OUTBOUND).map_err(error)?.into_values()
            .find(|g| g.peer == peer && scope.as_deref() == Some(&g.spaceId)
                && credentials.get(&g.pairingId).is_some_and(|r| r.deviceId == peer && r.pairingServiceVersion == PAIRING_SERVICE_VERSION))
            .ok_or_else(|| CoreLinkError::new("PEER_OUTBOUND_NOT_AUTHORIZED", "No paired or admitted Space return channel for node"))
    }

    /// Builds one scoped callback offer and installs its inbound credential before publication.
    async fn spaceChannelOfferRequest(&self, peer: &str, pairingId: &str) -> Result<Option<CoreCallRequest>, CoreLinkError> {
        let Some(scope) = self.spaceChannelScope(peer)? else { return Ok(None); };
        let Some(config) = self.state.store.hostConfig().map_err(error)? else { return Ok(None); };
        let Ok(socket) = config.bindAddress.parse::<std::net::SocketAddr>() else { return Ok(None); };
        let Some(transport) = self.state.listeners.lock().await.keys().copied().find(|t| isNetworkTransport(*t)) else { return Ok(None); };
        if socket.port() == 0 { return Err(error("Space callback listener has no bound port")); }
        let grant = {
            let _guard = self.state.mutation.lock().unwrap();
            let previous = self.state.store.records::<Grant>(INBOUND).map_err(error)?.into_values()
                .find(|g| g.peer == peer && g.spaceId == scope && g.pairingId == pairingId);
            let mut grant = previous.clone().unwrap_or(Grant {
                id: uuid::Uuid::new_v4().to_string(), spaceId: scope, peer: peer.into(), pairingId: pairingId.into(),
                secret: BASE64.encode(crypto::random()?), port: socket.port(), transport, endpoint: String::new(),
            });
            grant.port = socket.port(); grant.transport = transport;
            // Install before publishing: an immediate callback must be accepted.
            if previous.as_ref() != Some(&grant) {
                self.state.store.putRecord(INBOUND, &grant.id, &grant).map_err(error)?;
            }
            grant
        };
        let requestId = operit_link::nextCoreRouteRequestId("space-channel");
        Ok(Some(CoreCallRequest::new(requestId, TARGET, "offer", toCoreValue(Offer {
            id: grant.id, spaceId: grant.spaceId, secret: grant.secret, port: grant.port, transport: grant.transport,
        }).map_err(|e| error(e.to_string()))?)))
    }

    /// Publishes a scoped callback offer over the original authenticated channel.
    pub(super) async fn offerSpaceChannel(&self, peer: &str, pairingId: &str, channel: &Arc<Channel>) -> Result<(), CoreLinkError> {
        let Some(request) = self.spaceChannelOfferRequest(peer, pairingId).await? else { return Ok(()); };
        let requestId = request.requestId.0.clone();
        match channel.exchange(CoreLinkRequest::Call(request)).await? {
            CoreLinkResponse::Call(response) if response.requestId.0 == requestId => match response.result {
                Ok(_) => Ok(()),
                Err(e) if e.code == "SPACE_CHANNEL_NOT_ADMITTED" => Ok(()),
                Err(e) => Err(e),
            },
            _ => Err(error("Space channel offer response mismatch")),
        }
    }

    /// Publishes a scoped callback offer through a multiplexed channel already in the pool.
    pub(super) async fn offerSpaceChannelMultiplexed(&self, peer: &str, pairingId: &str, channel: &Arc<MultiplexedChannel>) -> Result<bool, CoreLinkError> {
        let Some(request) = self.spaceChannelOfferRequest(peer, pairingId).await? else { return Ok(false); };
        let requestId = request.requestId.0.clone();
        match channel.exchange(CoreLinkRequest::Call(request)).await? {
            CoreLinkResponse::Call(response) if response.requestId.0 == requestId => match response.result {
                Ok(_) => Ok(true),
                Err(e) if e.code == "SPACE_CHANNEL_NOT_ADMITTED" => Ok(false),
                Err(e) => Err(e),
            },
            _ => Err(error("Space channel offer response mismatch")),
        }
    }

    pub(super) fn acceptSpaceChannel(&self, peer: &str, pairingId: &str, raw: &Arc<dyn PeerConnection>, request: &CoreCallRequest) -> Result<CoreValue, CoreLinkError> {
        if request.methodName != "offer" { return Err(error("Unknown Space channel operation")); }
        let offer: Offer = fromCoreValue(request.args.clone()).map_err(|e| error(e.to_string()))?;
        let _guard = self.state.mutation.lock().unwrap();
        if self.spaceChannelScope(peer)?.as_deref() != Some(&offer.spaceId) {
            return Err(CoreLinkError::new("SPACE_CHANNEL_NOT_ADMITTED", "Both devices must be admitted to this Space"));
        }
        if !self.inboundCredentials()?.get(pairingId).is_some_and(|r| r.deviceId == peer && r.pairingServiceVersion == PAIRING_SERVICE_VERSION) {
            return Err(error("A Space channel offer requires the original authenticated pairing"));
        }
        if offer.id.is_empty() || offer.port == 0 || !isNetworkTransport(offer.transport)
            || BASE64.decode(&offer.secret).map_err(|_| error("Invalid Space channel credential"))?.len() != 32 {
            return Err(error("Invalid Space channel offer"));
        }
        let address = std::net::SocketAddr::new(raw.remoteAddress().ok_or_else(|| error("Space channel requires an observed peer address"))?.ip(), offer.port);
        let endpoint = match offer.transport {
            PeerTransport::Http => format!("http://{address}/link"),
            PeerTransport::WebSocket => format!("ws://{address}/link"),
            _ => address.to_string(),
        };
        let grant = Grant { id: offer.id, spaceId: offer.spaceId, peer: peer.into(), pairingId: pairingId.into(), secret: offer.secret,
            port: offer.port, transport: offer.transport, endpoint };
        let previous = self.state.store.records::<Grant>(OUTBOUND).map_err(error)?;
        if previous.get(&grant.id).is_some_and(|g| g.peer != peer || g.spaceId != grant.spaceId || g.pairingId != pairingId) {
            return Err(error("Space channel credential identity conflict"));
        }
        if previous.get(&grant.id) != Some(&grant) {
            self.state.store.putRecord(OUTBOUND, &grant.id, &grant).map_err(error)?;
            operit_util::AppLogger::AppLogger::i("SpaceChannel", &format!("return channel established space={} peer={} transport={:?}", grant.spaceId, peer, grant.transport));
            self.changed();
        }
        Ok(CoreValue::Null)
    }

    pub(super) fn removeSpaceChannels(&self, peer: &str) -> Result<(), CoreLinkError> {
        for path in [INBOUND, OUTBOUND] {
            for (id, grant) in self.state.store.records::<Grant>(path).map_err(error)? {
                if grant.peer == peer { self.state.store.deleteRecord(path, &id).map_err(error)?; }
            }
        }
        Ok(())
    }
}
