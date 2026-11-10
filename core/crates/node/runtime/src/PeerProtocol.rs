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

    pub(crate) fn requireAdvertised(protocol: Option<&Self>, phase: &str) -> Result<(), CoreLinkError> {
        let result = protocol.ok_or_else(|| CoreLinkError::new("PEER_PROTOCOL_UPGRADE_REQUIRED",
            "Peer message is missing the required protocol identity"))
            .and_then(Self::requireCompatible);
        result.map_err(|mut error| {
            operit_util::AppLogger::AppLogger::w("RuntimePeerService", &format!(
                "protocol.rejected phase={phase:?} localProtocol=v{VERSION} remoteProtocol={:?} code={}",
                protocol.map(|value| value.version), error.code));
            // A missing field does not establish that an installed app is outdated.
            // Keep the validation phase visible even when the caller only displays toString().
            error.message = format!("{} (phase: {phase}; local protocol v{VERSION})", error.message);
            error.details = Some(operit_link::toCoreValue(serde_json::json!({
                "phase": phase,
                "localProtocol": VERSION,
                "remoteProtocol": protocol.map(|value| value.version),
            })).expect("protocol diagnostics contain only scalar values"));
            error
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_identity_diagnostics_distinguish_request_reply_and_projection() {
        for phase in ["received hello", "received hello reply", "space projection reply"] {
            let failure = PeerProtocol::requireAdvertised(None, phase).unwrap_err();
            assert_eq!(failure.code, "PEER_PROTOCOL_UPGRADE_REQUIRED");
            assert!(failure.message.contains(phase));
            assert!(failure.message.contains(&format!("local protocol v{VERSION}")));
            let details: serde_json::Value = operit_link::fromCoreValue(failure.details.unwrap()).unwrap();
            assert_eq!(details["phase"], phase);
            assert_eq!(details["localProtocol"], VERSION);
            assert!(details["remoteProtocol"].is_null());
        }
        assert!(PeerProtocol::requireAdvertised(Some(&PeerProtocol::current()), "received hello").is_ok());
        let failure = PeerProtocol::requireAdvertised(Some(&PeerProtocol { version: VERSION + 1 }), "received hello reply").unwrap_err();
        assert_eq!(failure.code, "PEER_PROTOCOL_MISMATCH");
        let details: serde_json::Value = operit_link::fromCoreValue(failure.details.unwrap()).unwrap();
        assert_eq!(details["remoteProtocol"], VERSION + 1);
    }

    #[test]
    fn compatibility_requires_only_protocol_version() {
        assert!(PeerProtocol::current().requireCompatible().is_ok());
        let mut peer = PeerProtocol::current();
        peer.version += 1;
        assert_eq!(peer.requireCompatible().unwrap_err().code, "PEER_PROTOCOL_MISMATCH");
        assert_eq!(serde_json::to_value(PeerProtocol::current()).unwrap(), serde_json::json!({"version": VERSION}));
        assert_eq!(PeerProtocol::requireAdvertised(None, "test").unwrap_err().code, "PEER_PROTOCOL_UPGRADE_REQUIRED");
    }
}

#[cfg(test)]
#[path = "../build_support/peer_protocol.rs"]
mod protocol_guard_tests;
