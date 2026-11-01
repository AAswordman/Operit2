//! Versioned QR invitations authenticated through the existing SPAKE2 exchange.
use std::{collections::BTreeSet, net::{IpAddr, SocketAddr}};
use crate::NodeServices::PeerConnectionInfo;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use operit_link::CoreLinkError;
use operit_peer_link::PeerTransport;

pub const VERSION: u32 = 2;
const PREFIX: &str = "operit://connect/v2/";
const MAX_CODE_BYTES: usize = 4096;
const MAX_ADDRESSES: usize = 8;

/// Keeps invitation credentials inside the runtime rather than UI inspection DTOs.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionInvitation {
    #[serde(rename = "p")] pub info: PeerConnectionInfo,
    #[serde(rename = "i")] pub invitationId: String,
    #[serde(rename = "s")] pub secret: String,
}

/// Identifies malformed connection codes consistently across all clients.
fn invalid(message: impl Into<String>) -> CoreLinkError {
    CoreLinkError::new("INVALID_CONNECTION_CODE", message)
}

/// Checks one structural invariant without duplicating error construction.
fn require(valid: bool, message: &str) -> Result<(), CoreLinkError> {
    if valid { Ok(()) } else { Err(invalid(message)) }
}

/// Encodes coordinates and a receiver-issued invitation into one QR payload.
pub fn encode(invitation: &ConnectionInvitation) -> Result<String, CoreLinkError> {
    validate(invitation, "")?;
    let bytes = serde_json::to_vec(invitation).map_err(|error| invalid(error.to_string()))?;
    let code = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes));
    require(code.len() <= MAX_CODE_BYTES, "Connection code is too large")?;
    Ok(code)
}

/// Validates an invitation before any network or pairing state is created.
pub fn decode(code: &str, localNodeId: &str) -> Result<ConnectionInvitation, CoreLinkError> {
    let code = code.trim();
    require(code.len() <= MAX_CODE_BYTES, "Connection code is too large")?;
    let payload = code.strip_prefix(PREFIX).ok_or_else(|| {
        if code.starts_with("operit://connect/") {
            CoreLinkError::new("CONNECTION_CODE_VERSION", "This connection code requires a different app version")
        } else { invalid("This is not an Operit device connection code") }
    })?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).map_err(|_| invalid("Connection code is damaged"))?;
    let invitation = serde_json::from_slice(&bytes).map_err(|_| invalid("Connection code has an invalid format"))?;
    validate(&invitation, localNodeId)?;
    Ok(invitation)
}

/// Validates identity, invitation entropy and every advertised endpoint together.
fn validate(invitation: &ConnectionInvitation, localNodeId: &str) -> Result<(), CoreLinkError> {
    let info = &invitation.info;
    if info.version != VERSION {
        return Err(CoreLinkError::new("CONNECTION_CODE_VERSION", "Unsupported connection code version"));
    }
    if info.nodeId == localNodeId {
        return Err(CoreLinkError::new("CONNECTION_CODE_SELF", "This connection code belongs to this device"));
    }
    require(!info.nodeId.is_empty() && info.nodeId.len() <= 160
        && !info.nodeId.chars().any(|ch| ch.is_whitespace() || ch.is_control() || "/\\?#".contains(ch)),
        "Connection code has an invalid device identity")?;
    require(info.displayName.len() <= 256 && !info.displayName.chars().any(char::is_control),
        "Connection code has an invalid device name")?;
    require((1..=MAX_ADDRESSES).contains(&info.addresses.len()), "Connection code must contain between one and eight network addresses")?;
    uuid::Uuid::parse_str(&invitation.invitationId).map_err(|_| invalid("Invalid pairing invitation identity"))?;
    let secret = URL_SAFE_NO_PAD.decode(&invitation.secret).map_err(|_| invalid("Invalid pairing invitation secret"))?;
    require(secret.len() == 32, "Invalid pairing invitation secret")?;
    let mut unique = BTreeSet::new();
    for address in &info.addresses {
        endpointTransport(address)?;
        require(unique.insert(address), "Connection code contains duplicate addresses")?;
    }
    Ok(())
}

/// Accepts only reachable network coordinates, excluding credentials and URL parameters.
pub fn endpointTransport(address: &str) -> Result<PeerTransport, CoreLinkError> {
    require(address.len() <= 320 && !address.chars().any(|ch| ch.is_whitespace() || ch.is_control()), "Invalid connection address")?;
    let url = url::Url::parse(address).map_err(|_| invalid("Invalid connection address"))?;
    let transport = match url.scheme() {
        "http" | "https" => PeerTransport::Http,
        "ws" | "wss" => PeerTransport::WebSocket,
        "tcp" => PeerTransport::Tcp,
        _ => return Err(invalid("Connection code contains an unsupported transport")),
    };
    require(url.username().is_empty() && url.password().is_none() && url.query().is_none() && url.fragment().is_none()
        && matches!(url.path(), "" | "/") && matches!(url.port_or_known_default(), Some(1..=65535)),
        "Connection address must contain only a host and a valid port")?;
    let reachable = match url.host() {
        Some(url::Host::Ipv4(ip)) => usableAddress(IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => usableAddress(IpAddr::V6(ip)),
        Some(url::Host::Domain(host)) => !host.is_empty() && host != "localhost" && !host.ends_with(".localhost")
            && host.parse::<IpAddr>().map(usableAddress).unwrap_or(true),
        None => false,
    };
    require(reachable, "Connection address is not reachable from another device")?;
    Ok(transport)
}

/// Excludes addresses that cannot be copied between devices without an interface scope.
fn usableAddress(address: IpAddr) -> bool {
    !address.is_unspecified() && !address.is_loopback() && !address.is_multicast() && match address {
        IpAddr::V4(ip) => !ip.is_link_local() && !ip.is_broadcast(),
        IpAddr::V6(ip) => !ip.is_unicast_link_local(),
    }
}

/// Publishes live network sockets with deterministic IPv4 and transport ordering.
pub fn listenerAddresses(listeners: &[(PeerTransport, SocketAddr)], interfaces: &[IpAddr]) -> Result<Vec<String>, CoreLinkError> {
    let mut candidates = BTreeSet::new();
    for (transport, socket) in listeners {
        let scheme = match transport {
            PeerTransport::Http => "http",
            PeerTransport::WebSocket => "ws",
            PeerTransport::Tcp => "tcp",
            _ => continue,
        };
        if socket.port() == 0 { continue; }
        let addresses = interfaces.iter().copied()
            .filter(|ip| socket.ip().is_unspecified() && ip.is_ipv4() == socket.is_ipv4())
            .chain(std::iter::once(socket.ip())).filter(|ip| usableAddress(*ip));
        for ip in addresses {
            candidates.insert((ip.is_ipv6(), *transport, ip, format!("{scheme}://{}", SocketAddr::new(ip, socket.port()))));
        }
    }
    let addresses: Vec<_> = candidates.into_iter().take(MAX_ADDRESSES).map(|(_, _, _, address)| address).collect();
    if addresses.is_empty() {
        return Err(CoreLinkError::new("PAIRING_QR_UNAVAILABLE", "No reachable network listener. Enable receiving connections and check the network."));
    }
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Supplies a complete invitation for coordinate and credential validation.
    fn invitation() -> ConnectionInvitation {
        ConnectionInvitation {
            info: PeerConnectionInfo { version: VERSION, nodeId: "core-target".into(), displayName: "我的电脑".into(),
                addresses: vec!["http://192.168.1.5:37195".into(), "tcp://[fd00::5]:5712".into()] },
            invitationId: uuid::Uuid::new_v4().to_string(), secret: URL_SAFE_NO_PAD.encode([7; 32]),
        }
    }

    /// Round trips both the public coordinates and the private invitation without accepting self-pairing.
    #[test]
    fn round_trip_preserves_identity_ipv6_and_invitation_secret() {
        let expected = invitation();
        let code = encode(&expected).unwrap();
        let decoded = decode(&code, "core-other").unwrap();
        assert_eq!(decoded.info, expected.info);
        assert_eq!((decoded.invitationId, decoded.secret), (expected.invitationId, expected.secret));
        assert_eq!(decode(&code, &expected.info.nodeId).unwrap_err().code, "CONNECTION_CODE_SELF");
    }

    /// Rejects foreign, legacy, damaged and oversized codes before dialing.
    #[test]
    fn rejects_foreign_damaged_oversized_and_future_codes() {
        for code in ["https://example.com", "operit://connect/v1/abc", "operit://connect/v3/abc", "operit://connect/v2/!", &"x".repeat(MAX_CODE_BYTES + 1)] {
            assert!(decode(code, "local").is_err(), "{code}");
        }
        let valid = serde_json::to_value(invitation()).unwrap();
        for (path, value) in [
            ("/p/version", serde_json::json!(VERSION + 1)), ("/p/nodeId", serde_json::json!("")),
            ("/p/addresses", serde_json::json!([])), ("/p/addresses", serde_json::json!(["tcp://host:1", "tcp://host:1"])),
            ("/i", serde_json::json!("invalid")), ("/s", serde_json::json!(URL_SAFE_NO_PAD.encode([7; 6]))),
        ] {
            let mut malformed = valid.clone();
            *malformed.pointer_mut(path).unwrap() = value;
            let code = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(serde_json::to_vec(&malformed).unwrap()));
            assert!(decode(&code, "local").is_err(), "{path}");
            assert!(encode(&serde_json::from_value(malformed).unwrap()).is_err(), "{path}");
        }
        let mut unknown = valid;
        unknown["token"] = "secret".into();
        let code = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(serde_json::to_vec(&unknown).unwrap()));
        assert!(decode(&code, "local").is_err());
    }

    /// Rejects non-network, credential-bearing and locally scoped endpoint coordinates.
    #[test]
    fn rejects_unsafe_or_non_network_endpoints_before_any_dial() {
        for address in [
            "file:///etc/passwd", "serial://tty", "http://user:secret@host:80", "http://host:80/path",
            "http://host:80?token=secret", "tcp://0.0.0.0:12", "http://127.0.0.1:12", "tcp://[::1]:12",
            "tcp://[fe80::1]:12", "http://host:0", "http://localhost:80", "http://169.254.1.2:80",
        ] { assert!(endpointTransport(address).is_err(), "{address}"); }
    }

    /// Resolves wildcard sockets against current usable interfaces and actual bound ports.
    #[test]
    fn wildcard_listeners_use_current_interfaces_and_actual_ports() {
        let listeners = [(PeerTransport::Http, "0.0.0.0:5712".parse().unwrap()), (PeerTransport::Tcp, "[::]:5832".parse().unwrap())];
        let interfaces = ["127.0.0.1", "0.0.0.0", "192.168.1.4", "169.254.1.2", "::1", "fd00::4", "fe80::1"].map(|ip| ip.parse().unwrap());
        assert_eq!(listenerAddresses(&listeners, &interfaces).unwrap(), vec!["http://192.168.1.4:5712", "tcp://[fd00::4]:5832"]);
        for listeners in [vec![], vec![(PeerTransport::Http, "127.0.0.1:5712".parse().unwrap())]] {
            assert!(listenerAddresses(&listeners, &interfaces).is_err());
        }
        let bound = [(PeerTransport::Http, "192.168.1.8:5712".parse().unwrap())];
        assert_eq!(listenerAddresses(&bound, &interfaces).unwrap(), vec!["http://192.168.1.8:5712"]);
    }
}
