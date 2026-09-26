use operit_edge_transport::{
    AuthenticatedLinkChannel, EdgePairingAuthority, EdgePeerLink, EdgeSpaceRouteClient, LinkChannel,
};
use std::sync::Arc;

/// Handles the first frame and then serves one authenticated EdgeLink session.
pub async fn handleChannel(
    authority: Arc<EdgePairingAuthority>,
    channel: Arc<dyn LinkChannel>,
) -> Result<(), String> {
    let first = channel
        .receive()
        .await?
        .ok_or_else(|| "Edge Link carrier closed".to_string())?;
    match &first.payload {
        operit_link::LinkFramePayload::PairStart(request) => {
            log::info!("Edge pairing: PairStart decoded");
            let session = authority
                .pairFromStart(channel.clone(), request.clone())
                .await?;
            log::info!("Edge pairing: PairFinish accepted");
            #[cfg(target_os = "espidf")]
            crate::logRuntimeHealth("edge-paired");
            serveAuthenticatedSession(channel, session).await
        }
        operit_link::LinkFramePayload::PairFinish(request) => {
            // The CLI deliberately runs pair-start and pair-finish as separate
            // processes. Therefore PairFinish may be the first frame on a new
            // carrier; the authority keeps the pending transaction by ID.
            let session = authority
                .pairFinishFromRequest(channel.clone(), request.clone())
                .await?;
            serveAuthenticatedSession(channel, session).await
        }
        operit_link::LinkFramePayload::Authenticated { .. } => {
            let (session, inner) = authority.authenticateFrame(&first)?;
            let peerId = session.peerDeviceId.clone();
            let authenticated = AuthenticatedLinkChannel::new(channel, session);
            installSpaceRoute(authenticated, inner, &peerId).await?;
            Ok(())
        }
        _ => Err("Edge Link connection did not start with pairing or authentication".to_string()),
    }
}

async fn serveAuthenticatedSession(
    channel: Arc<dyn LinkChannel>,
    session: operit_edge_transport::EdgeSession,
) -> Result<(), String> {
    let peerId = session.peerDeviceId.clone();
    let authenticated = AuthenticatedLinkChannel::new(channel, session);
    let context = tokio::time::timeout(std::time::Duration::from_secs(30), authenticated.receive())
        .await
        .map_err(|_| "Space admission timed out".to_string())??
        .ok_or_else(|| "Space admission context was not received".to_string())?;
    installSpaceRoute(authenticated, context, &peerId).await
}

async fn installSpaceRoute(
    channel: Arc<dyn LinkChannel>,
    frame: operit_link::LinkFrame,
    peerId: &str,
) -> Result<(), String> {
    let operit_link::LinkFramePayload::SpaceContext {
        spaceId,
        adjacentNodeId,
        ttl,
        chatId,
    } = frame.payload
    else {
        return Err("Edge session did not begin with authenticated Space admission".to_string());
    };
    if spaceId.trim().is_empty() || chatId.trim().is_empty() || adjacentNodeId != peerId || ttl == 0
    {
        return Err("Invalid authenticated Space route context".to_string());
    }
    let peer = EdgePeerLink::new(channel);
    let client = EdgeSpaceRouteClient::throughAdjacent(
        peer.clone(),
        spaceId,
        peerId.to_string(),
        adjacentNodeId,
        ttl,
    );
    operit_link::installCoreRouteRuntime(Arc::new(client.clone()));
    crate::edge_chat::install(client, chatId);
    // Keep the UART session owner alive while PeerLink owns receive(); the
    // listener must not compete with it for the next authenticated frame.
    while peer.isConnected() {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Ok(())
}
