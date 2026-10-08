//! Configured limits, not allocator telemetry or historical board measurements.
use operit_node_runtime::HostRuntimePeerService::PeerRuntimeLimits;

pub fn snapshot() -> serde_json::Value {
    let limits = PeerRuntimeLimits::constrained();
    serde_json::json!({
        "profile": "esp32-2432s028",
        "kind": "configured-limits",
        "liveTelemetry": false,
        "maxPeerMessageBytes": limits.maxMessageBytes,
        "incomingSessions": limits.incomingSessions,
        "pendingPairings": limits.pendingPairings,
        "concurrentProbes": limits.concurrentProbes,
        "enforcedOperations": ["framed-peer-send", "framed-peer-receive"],
        "note": "Peer limits match the firmware. Desktop allocations are not board heap telemetry; UI RAM and Wasm stack are reported by the renderer separately."
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn reports_only_enforced_limits_not_fake_heap() {
        let snapshot = super::snapshot();
        assert_eq!(snapshot["maxPeerMessageBytes"], 8192);
        assert_eq!(snapshot["liveTelemetry"], false);
        for field in ["freeHeap", "largest8BitBlock", "baselineUiPool", "staticDram"] {
            assert!(snapshot.get(field).is_none(), "{field}");
        }
    }
}
