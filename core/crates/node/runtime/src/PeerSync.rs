//! Typed peer synchronization protocol. Local management targets are never exposed.
use operit_link::{CoreCallRequest, CoreLinkError, CoreValue};
use serde::{Deserialize, Serialize};

pub(crate) const NODE_SYNC_TARGET: &str = "$node.sync";

/// The complete remote call surface. Unknown methods fail deserialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PeerSyncMethod {
    CoreVersion,
    DeviceSpace,
    SyncClock,
    SyncOperationsSince,
    SyncApplyOperations,
    SyncBlobExists,
    SyncReadBlobChunk,
    SyncApplyImmediateBindingOperation,
}

impl PeerSyncMethod {
    /// Device-space inspection and immediate execution binding are control-plane
    /// calls, not an invitation to replicate persistent business data.
    pub(crate) fn requiresStorageProvider(self) -> bool {
        matches!(self, Self::SyncClock | Self::SyncOperationsSince
            | Self::SyncApplyOperations | Self::SyncBlobExists | Self::SyncReadBlobChunk)
    }

    pub(crate) fn request(self, requestId: String, args: CoreValue) -> CoreCallRequest {
        // This unit enum always serializes to a string under its wire naming convention.
        let name = serde_json::to_value(self).expect("peer sync method must serialize");
        CoreCallRequest::new(
            requestId,
            NODE_SYNC_TARGET,
            name.as_str().expect("peer sync method must be a string"),
            args,
        )
    }

    pub(crate) fn fromRequest(request: &CoreCallRequest) -> Result<Self, CoreLinkError> {
        let method: Self = serde_json::from_value(serde_json::Value::String(
            request.methodName.clone(),
        ))
        .map_err(|_| {
            CoreLinkError::new(
                "PEER_SYNC_METHOD_DENIED",
                "Method is not a peer synchronization operation",
            )
        })?;
        if method == Self::SyncApplyOperations {
            let args: ApplyOperations =
                operit_link::fromCoreValue(request.args.clone()).map_err(|_| {
                    CoreLinkError::new(
                        "PEER_SYNC_INVALID_OPERATIONS",
                        "Remote synchronization requires an operation array",
                    )
                })?;
            // Bootstrap forceApply is local-only; it cannot be represented by this DTO.
            let _operations = args.operations;
        }
        Ok(method)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyOperations {
    operations: Vec<operit_store::SyncOperationStore::SyncOperation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PeerSyncPushMethod {
    SyncReceiveBlob,
}

impl PeerSyncPushMethod {
    pub(crate) fn fromRequest(
        request: &operit_link::CorePushRequest,
    ) -> Result<Self, CoreLinkError> {
        serde_json::from_value(serde_json::Value::String(request.methodName.clone())).map_err(
            |_| {
                CoreLinkError::new(
                    "PEER_SYNC_METHOD_DENIED",
                    "Only synchronization blob pushes are allowed",
                )
            },
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use operit_link::toCoreValue;
    use serde_json::json;

    #[test]
    fn peer_sync_methods_round_trip_through_standard_link_requests() {
        for method in [
            PeerSyncMethod::CoreVersion,
            PeerSyncMethod::DeviceSpace,
            PeerSyncMethod::SyncClock,
            PeerSyncMethod::SyncOperationsSince,
            PeerSyncMethod::SyncApplyOperations,
            PeerSyncMethod::SyncBlobExists,
            PeerSyncMethod::SyncReadBlobChunk,
            PeerSyncMethod::SyncApplyImmediateBindingOperation,
        ] {
            let request = method.request(
                "test".into(),
                toCoreValue(json!({"operations":[]})).unwrap(),
            );
            assert_eq!(request.target, NODE_SYNC_TARGET);
            assert_eq!(PeerSyncMethod::fromRequest(&request).unwrap(), method);
        }
    }

    #[test]
    fn peer_sync_does_not_expose_management_or_bootstrap_override() {
        let request =
            CoreCallRequest::new("test", NODE_SYNC_TARGET, "startPairing", CoreValue::Null);
        assert!(PeerSyncMethod::fromRequest(&request).is_err());
        for args in [
            json!({"operations":{"operations":[],"forceApply":true}}),
            json!({"operations":[],"forceApply":true}),
        ] {
            let request = PeerSyncMethod::SyncApplyOperations
                .request("test".into(), toCoreValue(args).unwrap());
            assert!(PeerSyncMethod::fromRequest(&request).is_err());
        }
    }

    #[test]
    fn peer_sync_push_accepts_only_blob_transfer() {
        let request =
            operit_link::CorePushRequest::new("test", NODE_SYNC_TARGET, "syncReceiveBlob");
        assert_eq!(
            PeerSyncPushMethod::fromRequest(&request).unwrap(),
            PeerSyncPushMethod::SyncReceiveBlob
        );
        let request = operit_link::CorePushRequest::new("test", NODE_SYNC_TARGET, "startListening");
        assert!(PeerSyncPushMethod::fromRequest(&request).is_err());
    }
}
