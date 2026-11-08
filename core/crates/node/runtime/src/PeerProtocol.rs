//! One compatibility identity for Core and Edge pairing, routing and synchronization.
use operit_link::CoreLinkError;
use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/peer_protocol_identity.rs"));

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerProtocol {
    pub version: u32,
}

impl PeerProtocol {
    pub fn current() -> Self {
        Self { version: VERSION }
    }

    /// Only the protocol version controls interoperability. The fingerprint is
    /// checked during compilation and is never embedded in protocol messages.
    pub fn requireCompatible(&self) -> Result<(), CoreLinkError> {
        if self.version != VERSION {
            return Err(CoreLinkError::new("PEER_PROTOCOL_MISMATCH", format!(
                "Peer protocol mismatch: local v{VERSION}, remote v{}; update the incompatible endpoint",
                self.version)));
        }
        Ok(())
    }

    pub(crate) fn requireAdvertised(protocol: Option<&Self>) -> Result<(), CoreLinkError> {
        protocol.ok_or_else(|| CoreLinkError::new("PEER_PROTOCOL_UPGRADE_REQUIRED",
            "Peer does not advertise a protocol identity; update its Core or Edge firmware"))?
            .requireCompatible()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_requires_only_protocol_version() {
        assert!(PeerProtocol::current().requireCompatible().is_ok());
        let mut peer = PeerProtocol::current();
        peer.version += 1;
        assert_eq!(peer.requireCompatible().unwrap_err().code, "PEER_PROTOCOL_MISMATCH");
        assert_eq!(serde_json::to_value(PeerProtocol::current()).unwrap(), serde_json::json!({"version": VERSION}));
        assert_eq!(PeerProtocol::requireAdvertised(None).unwrap_err().code, "PEER_PROTOCOL_UPGRADE_REQUIRED");
    }
}

#[cfg(test)]
#[path = "../build_support/peer_protocol.rs"]
mod protocol_guard_tests;
