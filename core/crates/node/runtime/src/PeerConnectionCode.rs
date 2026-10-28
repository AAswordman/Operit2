//! Versioned public coordinates shared by Flutter and terminal QR codes.
//! A code carries no credential: the ordinary pairing handshake and Space approval still apply.
use std::collections::BTreeSet;
use std::net::{IpAddr, SocketAddr};

use crate::NodeServices::PeerConnectionInfo;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use operit_link::CoreLinkError;
use operit_peer_link::PeerTransport;

pub const VERSION: u32 = 1;
const PREFIX: &str = "operit://connect/v1/";
const MAX_CODE_BYTES: usize = 4096;
const MAX_ADDRESSES: usize = 8;

// Compact wire keys keep a multi-address code readable in an ordinary terminal.
// The URI carries the protocol version; UI DTO field names are independent.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireInfo {
    n: String,
    d: String,
    a: Vec<String>,
}

fn invalid(message: impl Into<String>) -> CoreLinkError {
    CoreLinkError::new("INVALID_CONNECTION_CODE", message)
}

pub fn encode(info: &PeerConnectionInfo) -> Result<String, CoreLinkError> {
    validate(info, "")?;
    let bytes = serde_json::to_vec(&WireInfo {
        n: info.nodeId.clone(),
        d: info.displayName.clone(),
        a: info.addresses.clone(),
    })
    .map_err(|error| invalid(error.to_string()))?;
    let code = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes));
    if code.len() > MAX_CODE_BYTES {
        return Err(invalid("Connection code is too large"));
    }
    Ok(code)
}

pub fn decode(code: &str, localNodeId: &str) -> Result<PeerConnectionInfo, CoreLinkError> {
    let code = code.trim();
    if code.len() > MAX_CODE_BYTES {
        return Err(invalid("Connection code is too large"));
    }
    let payload = code.strip_prefix(PREFIX).ok_or_else(|| {
        if code.starts_with("operit://connect/") {
            CoreLinkError::new(
                "CONNECTION_CODE_VERSION",
                "This connection code requires a different app version",
            )
        } else {
            invalid("This is not an Operit device connection code")
        }
    })?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| invalid("Connection code is damaged"))?;
    let wire: WireInfo = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("Connection code has an invalid format"))?;
    let info = PeerConnectionInfo {
        version: VERSION,
        nodeId: wire.n,
        displayName: wire.d,
        addresses: wire.a,
    };
    validate(&info, localNodeId)?;
    Ok(info)
}

fn validate(info: &PeerConnectionInfo, localNodeId: &str) -> Result<(), CoreLinkError> {
    if info.version != VERSION {
        return Err(CoreLinkError::new(
            "CONNECTION_CODE_VERSION",
            "Unsupported connection code version",
        ));
    }
    if info.nodeId.is_empty()
        || info.nodeId.len() > 160
        || info
            .nodeId
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control() || "/\\?#".contains(ch))
    {
        return Err(invalid("Connection code has an invalid device identity"));
    }
    if info.nodeId == localNodeId {
        return Err(CoreLinkError::new(
            "CONNECTION_CODE_SELF",
            "This connection code belongs to this device",
        ));
    }
    if info.displayName.len() > 256 || info.displayName.chars().any(char::is_control) {
        return Err(invalid("Connection code has an invalid device name"));
    }
    if info.addresses.is_empty() || info.addresses.len() > MAX_ADDRESSES {
        return Err(invalid(
            "Connection code must contain between one and eight network addresses",
        ));
    }
    let mut unique = BTreeSet::new();
    for address in &info.addresses {
        endpointTransport(address)?;
        if !unique.insert(address) {
            return Err(invalid("Connection code contains duplicate addresses"));
        }
    }
    Ok(())
}

/// Only network peer endpoints are accepted. No credentials, file access, shell schemes or URL parameters.
pub fn endpointTransport(address: &str) -> Result<PeerTransport, CoreLinkError> {
    if address.len() > 320
        || address
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
    {
        return Err(invalid("Invalid connection address"));
    }
    let url = url::Url::parse(address).map_err(|_| invalid("Invalid connection address"))?;
    let transport = match url.scheme() {
        "http" | "https" => PeerTransport::Http,
        "ws" | "wss" => PeerTransport::WebSocket,
        "tcp" => PeerTransport::Tcp,
        _ => return Err(invalid("Connection code contains an unsupported transport")),
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
        || !matches!(url.port_or_known_default(), Some(1..=65535))
    {
        return Err(invalid(
            "Connection address must contain only a host and a valid port",
        ));
    }
    match url.host() {
        Some(url::Host::Ipv4(ip)) if usableAddress(IpAddr::V4(ip)) => {}
        Some(url::Host::Ipv6(ip)) if usableAddress(IpAddr::V6(ip)) => {}
        Some(url::Host::Domain(host))
            if !host.is_empty()
                && host != "localhost"
                && !host.ends_with(".localhost")
                && host.parse::<IpAddr>().map(usableAddress).unwrap_or(true) => {}
        _ => {
            return Err(invalid(
                "Connection address is not reachable from another device",
            ))
        }
    }
    Ok(transport)
}

fn usableAddress(address: IpAddr) -> bool {
    if address.is_unspecified() || address.is_loopback() || address.is_multicast() {
        return false;
    }
    match address {
        IpAddr::V4(ip) => !ip.is_link_local() && !ip.is_broadcast(),
        // An IPv6 link-local address needs the receiving device's interface scope.
        // A sender's scope ID cannot be copied to another device in a QR code.
        IpAddr::V6(ip) => !ip.is_unicast_link_local(),
    }
}

pub fn listenerAddresses(
    listeners: &[(PeerTransport, SocketAddr)],
    interfaces: &[IpAddr],
) -> Result<Vec<String>, CoreLinkError> {
    let mut candidates = BTreeSet::new();
    for (transport, socket) in listeners {
        let scheme = match transport {
            PeerTransport::Http => "http",
            PeerTransport::WebSocket => "ws",
            PeerTransport::Tcp => "tcp",
            _ => continue,
        };
        if socket.port() == 0 {
            continue;
        }
        let addresses = if socket.ip().is_unspecified() {
            interfaces
                .iter()
                .copied()
                .filter(|ip| ip.is_ipv4() == socket.is_ipv4())
                .collect::<Vec<_>>()
        } else {
            vec![socket.ip()]
        };
        for ip in addresses.into_iter().filter(|ip| usableAddress(*ip)) {
            let endpoint = format!("{scheme}://{}", SocketAddr::new(ip, socket.port()));
            // Prefer IPv4 and HTTP, with deterministic ordering across refreshes.
            candidates.insert((ip.is_ipv6(), *transport, ip, endpoint));
        }
    }
    let addresses = candidates
        .into_iter()
        .take(MAX_ADDRESSES)
        .map(|(_, _, _, address)| address)
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(CoreLinkError::new(
            "PAIRING_QR_UNAVAILABLE",
            "No reachable network listener. Enable receiving connections and check the network.",
        ));
    }
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn info() -> PeerConnectionInfo {
        PeerConnectionInfo {
            version: VERSION,
            nodeId: "core-target".into(),
            displayName: "我的电脑".into(),
            addresses: vec![
                "http://192.168.1.5:37195".into(),
                "tcp://[fd00::5]:5712".into(),
            ],
        }
    }
    #[test]
    fn round_trip_preserves_identity_and_ipv6_without_credentials() {
        let expected = info();
        assert_eq!(
            decode(&encode(&expected).unwrap(), "core-other").unwrap(),
            expected
        );
        assert_eq!(
            decode(&encode(&expected).unwrap(), &expected.nodeId)
                .unwrap_err()
                .code,
            "CONNECTION_CODE_SELF"
        );
    }
    #[test]
    fn rejects_foreign_damaged_oversized_and_future_codes() {
        for code in [
            "https://example.com",
            "operit://connect/v1/!",
            "operit://connect/v2/abc",
            &"x".repeat(MAX_CODE_BYTES + 1),
        ] {
            assert!(decode(code, "local").is_err(), "{code}");
        }
        let mut future = info();
        future.version += 1;
        assert!(encode(&future).is_err());
        let code = encode(&info()).unwrap();
        let mut unknown: serde_json::Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(code.strip_prefix(PREFIX).unwrap())
                .unwrap(),
        )
        .unwrap();
        unknown["token"] = "secret".into();
        assert!(decode(
            &format!(
                "{PREFIX}{}",
                URL_SAFE_NO_PAD.encode(serde_json::to_vec(&unknown).unwrap())
            ),
            "local"
        )
        .is_err());
    }
    #[test]
    fn rejects_unsafe_or_non_network_endpoints_before_any_dial() {
        for address in [
            "file:///etc/passwd",
            "serial://tty",
            "http://user:secret@host:80",
            "http://host:80/path",
            "http://host:80?token=secret",
            "tcp://0.0.0.0:12",
            "http://127.0.0.1:12",
            "tcp://[::1]:12",
            "tcp://[fe80::1]:12",
            "http://host:0",
            "http://localhost:80",
            "http://169.254.1.2:80",
        ] {
            assert!(endpointTransport(address).is_err(), "{address}");
        }
        let mut empty = info();
        empty.addresses.clear();
        assert!(encode(&empty).is_err());
        let mut duplicate = info();
        duplicate.addresses.push(duplicate.addresses[0].clone());
        assert!(encode(&duplicate).is_err());
    }
    #[test]
    fn wildcard_listeners_use_current_interfaces_and_actual_ports() {
        let listeners = [
            (PeerTransport::Http, "0.0.0.0:5712".parse().unwrap()),
            (PeerTransport::Tcp, "[::]:5832".parse().unwrap()),
        ];
        let interfaces = [
            "127.0.0.1",
            "0.0.0.0",
            "192.168.1.4",
            "169.254.1.2",
            "::1",
            "fd00::4",
            "fe80::1",
        ]
        .map(|ip| ip.parse().unwrap());
        assert_eq!(
            listenerAddresses(&listeners, &interfaces).unwrap(),
            vec!["http://192.168.1.4:5712", "tcp://[fd00::4]:5832"]
        );
        assert!(listenerAddresses(&[], &interfaces).is_err());
        assert!(listenerAddresses(
            &[(PeerTransport::Http, "127.0.0.1:5712".parse().unwrap())],
            &interfaces
        )
        .is_err());
        assert_eq!(
            listenerAddresses(
                &[(PeerTransport::Http, "192.168.1.8:5712".parse().unwrap())],
                &interfaces
            )
            .unwrap(),
            vec!["http://192.168.1.8:5712"]
        );
    }
}
