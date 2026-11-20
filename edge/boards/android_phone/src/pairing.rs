use operit_edge_sdk::{
    runtime::{
        NodeServices::{PairedPeer, PeerEndpoint, PeerTransport},
        PeerConnectionCode::{self, ConnectionInvitation},
    },
    space::RuntimePeerService,
};
use std::time::Duration;

/// Parses the standard Space invitation and selects this board's supported transport.
pub(super) fn decode_invitation(code: &str, node_id: &str) -> Result<ConnectionInvitation, String> {
    let mut invitation = PeerConnectionCode::decode(code, node_id).map_err(|error| error.to_string())?;
    invitation.info.addresses.retain(|address| {
        matches!(PeerConnectionCode::endpointTransport(address), Ok(PeerTransport::Tcp))
    });
    if invitation.info.addresses.is_empty() {
        return Err("请在 Operit 设备空间开启 TCP 监听，再生成二维码".into());
    }
    Ok(invitation)
}

/// Authenticates the invitation on an advertised TCP address before any Space request.
pub(super) async fn pair_invitation(
    peers: &dyn RuntimePeerService,
    invitation: ConnectionInvitation,
) -> Result<PairedPeer, String> {
    let mut errors = Vec::new();
    for address in invitation.info.addresses {
        let attempt = peers.startQrPairing(
            PeerEndpoint { nodeId: invitation.info.nodeId.clone(), address },
            PeerTransport::Tcp,
            &invitation.invitationId,
        );
        match tokio::time::timeout(Duration::from_secs(4), attempt).await {
            Ok(Ok(pairing)) => {
                let result = peers.finishPairing(&pairing.pairingId, &invitation.secret).await;
                if result.is_err() {
                    peers.cancelPairing(&pairing.pairingId).await.map_err(|error| error.to_string())?;
                }
                return result.map_err(|error| error.to_string());
            }
            Ok(Err(error)) => errors.push(error.to_string()),
            Err(_) => errors.push("连接超时".into()),
        }
    }
    Err(format!("无法连接邀请设备\n{}", errors.join("\n")))
}
