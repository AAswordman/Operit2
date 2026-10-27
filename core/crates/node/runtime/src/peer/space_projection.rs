//! Bounded Space transport shared by peers that validate their own local authority.
#![allow(non_snake_case)]
use super::{NodeSpaceContext, CoreSpace, CoreSpaceDeviceProfile,
    CoreSpaceTopologyRecord, CoreSpaceStore, SyncOperation, space_departure};
use super::PeerSpaceSnapshot;
use operit_link::{CoreValue, fromCoreValue, toCoreValue, encodedLinkSize};
use operit_store::NetworkControlStore::SpaceControlExport;
use serde::{Deserialize, Serialize};

pub(crate) const VERSION: u32 = 1;
// Space payloads contain stored data only. Reserve space for routed Call/Reply,
// correlation, authenticated session framing and MessagePack container headers.
pub(crate) const ENVELOPE_BYTES: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ProjectionProtocol {
    pub version: u32,
    pub nodeId: String,
    pub endpoint: bool,
    pub authorityMode: String,
    pub maxMessageBytes: usize,
    pub snapshotPaging: bool,
    #[serde(default)]
    pub projectionReference: bool,
}
impl ProjectionProtocol {
    /// Validates peer identity and the explicitly advertised projection capabilities.
    pub fn validate(&self, peer: &str) -> Result<(), String> {
        if self.version != VERSION || self.nodeId != peer || self.authorityMode != "localReplay"
            || self.maxMessageBytes <= ENVELOPE_BYTES || (self.endpoint && !self.snapshotPaging)
            || (self.projectionReference && !self.snapshotPaging) {
            return Err("SPACE_PROTOCOL_INCOMPATIBLE: Unsupported projection protocol".into());
        }
        Ok(())
    }
}
#[derive(Deserialize)]
pub(crate) struct BoundedControlExchange {
    pub version: u32,
    pub maxMessageBytes: usize,
    pub operations: Vec<SyncOperation>,
}

/// Validates the explicit transport contract; peer errors terminate this exchange.
pub(crate) fn decodeProtocolResponse(
    peer: &str, result: Result<CoreValue, operit_link::CoreLinkError>,
) -> Result<ProjectionProtocol, String> {
    let value = result.map_err(|error|error.to_string())?;
    let protocol: ProjectionProtocol = fromCoreValue(value).map_err(|error|error.to_string())?;
    protocol.validate(peer)?;
    Ok(protocol)
}

#[derive(Serialize, Deserialize)]
struct ProtocolRequest { version: u32 }

pub(crate) fn negotiate(service: &dyn NodeSpaceContext, args: CoreValue) -> Result<CoreValue, String> {
    let request: ProtocolRequest = fromCoreValue(args).map_err(|error| error.to_string())?;
    if request.version != VERSION {
        return Err("SPACE_PROTOCOL_INCOMPATIBLE: Unsupported projection protocol version".into());
    }
    let peers = service.peers()?;
    let protocol = ProjectionProtocol { version: VERSION, nodeId: service.localNodeId(),
        endpoint: service.isSpaceEndpoint(), authorityMode: "localReplay".into(),
        maxMessageBytes: peers.maxMessageBytes(), snapshotPaging: true,
        projectionReference: service.isSpaceEndpoint() };
    protocol.validate(&protocol.nodeId)?;
    toCoreValue(protocol).map_err(|error| error.to_string())
}

/// Measures borrowed plain records; never serialize a consuming reply twice.
pub(crate) fn requirePayloadFits(value: &impl Serialize, maximum: usize) -> Result<(), String> {
    let size = encodedLinkSize(value).map_err(|error| error.to_string())?;
    if size.checked_add(ENVELOPE_BYTES).is_none_or(|size| size > maximum) {
        return Err(format!("SPACE_MESSAGE_LIMIT: Payload needs {size} bytes plus envelope; endpoint limit is {maximum}"));
    }
    Ok(())
}

/// Match the existing routed Call envelope using borrowed arguments. Include
/// actual identity lengths before applying the session/framing reservation.
#[cfg(any(test, feature = "full"))]
pub(crate) fn requireRoutedRequestFits(
    request: &operit_link::CoreCallRequest, space: &str, local: &str, peer: &str,
    ttl: u32, maximum: usize,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Route<'a> { spaceId: &'a str, originNodeId: &'a str, targetNodeId: &'a str,
        ttl: u32, routeKind: operit_link::RoutedCoreRequestKind, payload: &'a operit_link::CoreCallRequest }
    #[derive(Serialize)]
    struct Call<'a> { requestId: &'a operit_link::CoreRequestId, target: &'a str, methodName: &'a str, args: Route<'a> }
    #[derive(Serialize)]
    #[serde(tag = "type", content = "body")]
    enum Envelope<'a> { Call(Call<'a>) }
    requirePayloadFits(&Envelope::Call(Call { requestId: &request.requestId, target: &request.target,
        methodName: &request.methodName, args: Route { spaceId: space, originNodeId: local,
            targetNodeId: peer, ttl, routeKind: operit_link::RoutedCoreRequestKind::Target, payload: request } }), maximum)
}

pub(crate) fn requireEncodedFits(bytes: &[u8], maximum: usize) -> Result<(), String> {
    if bytes.len().checked_add(ENVELOPE_BYTES).is_none_or(|size| size > maximum) {
        return Err("SPACE_MESSAGE_LIMIT: Encoded Space request exceeds the endpoint limit".into());
    }
    Ok(())
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SnapshotHeader {
    pub space: CoreSpace,
    pub deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    pub topology: Vec<CoreSpaceTopologyRecord>,
    pub departures: Vec<SyncOperation>,
    pub controlRevision: String,
    pub controlCount: usize,
}
impl SnapshotHeader {
    /// Attaches the complete verified export to its correlated metadata.
    pub fn withOperations(self, mut controlOperations: Vec<SyncOperation>) -> PeerSpaceSnapshot {
        controlOperations.sort_by_key(operit_store::SyncOperationStore::SyncOperationOrder::fromOperation);
        PeerSpaceSnapshot { space: self.space, deviceProfiles: self.deviceProfiles,
            topology: self.topology, departures: self.departures, controlOperations }
    }
}
#[derive(Serialize, Deserialize)]
pub(crate) struct ControlPageRequest {
    pub spaceId: String,
    pub revision: String,
    pub after: Option<String>,
}

/// Projects snapshot metadata without loading the first control operation.
pub(crate) fn header(service: &dyn NodeSpaceContext, peer: &str) -> Result<SnapshotHeader, String> {
    let space = service.spaceStore().space()?;
    let manifest = service.networkControlStore().spaceControlManifest(&space.spaceId)?;
    let deviceProfiles = service.spaceStore().deviceProfiles()?.into_iter()
        .filter(|(node, _)| space.members.contains(node)).map(|(_, profile)| profile).collect();
    let topology = service.spaceStore().topologyRecords()?.into_iter()
        .filter(|(node, _)| space.members.contains(node)).map(|(_, record)| record).collect();
    let header = SnapshotHeader { space, deviceProfiles, topology,
        departures: space_departure::pending(service, peer)?,
        controlRevision: manifest.revision, controlCount: manifest.count };
    requirePayloadFits(&header, service.peers()?.maxMessageBytes())?;
    Ok(header)
}
/// Returns one operation correlated to the exact current export manifest.
pub(crate) fn controlPage(service: &dyn NodeSpaceContext, args: CoreValue) -> Result<CoreValue, String> {
    let request: ControlPageRequest = fromCoreValue(args).map_err(|error| error.to_string())?;
    if request.spaceId != service.spaceStore().space()?.spaceId {
        return Err("SPACE_EXPORT_CHANGED: Endpoint Space changed".into());
    }
    let page = service.networkControlStore().spaceControlExport(&request.spaceId,
        Some(&request.revision), request.after.as_deref())?;
    requirePayloadFits(&page, service.peers()?.maxMessageBytes())?;
    toCoreValue(page).map_err(|error| error.to_string())
}

/// Metadata refresh bound to the endpoint's complete, immutable policy export.
/// This cannot import authority, acknowledge departures, or migrate a Space.
#[derive(Serialize, Deserialize)]
pub(crate) struct ProjectionObservation {
    pub controlRevision: String,
    pub space: CoreSpace,
    pub deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    pub topology: Vec<CoreSpaceTopologyRecord>,
}

pub(crate) fn observeProjection(service: &dyn NodeSpaceContext, peer: &str, args: CoreValue) -> Result<CoreValue, String> {
    if !service.isSpaceEndpoint() { return Err("SPACE_PROTOCOL_INCOMPATIBLE: Projection references are endpoint-only".into()); }
    let observation: ProjectionObservation = fromCoreValue(args).map_err(|error| error.to_string())?;
    let local = service.spaceStore().space()?;
    if observation.space.spaceId != local.spaceId || !space_departure::pending(service, peer)?.is_empty() {
        return Err("SPACE_EXPORT_CHANGED: A complete Space exchange is required".into());
    }
    CoreSpaceStore::validateSpaceProfiles(&observation.space, &observation.deviceProfiles)?;
    if !observation.space.members.iter().any(|node| node == peer) {
        return Err("Paired device is not present in its announced device space".into());
    }
    // Verify the exact policy before any publication. Opening this export also
    // validates all immutable envelopes, including unknown payload fields.
    let manifest = service.networkControlStore().spaceControlManifest(&local.spaceId)?;
    if manifest.revision != observation.controlRevision {
        return Err("SPACE_EXPORT_CHANGED: Space control projection changed".into());
    }
    let policy = service.networkControlStore().currentState()?;
    for node in &observation.space.members {
        if !policy.memberNodeIds.contains(node) {
            return Err(format!("Space snapshot contains an unauthorized member: {node}"));
        }
    }
    drop(policy);
    service.spaceStore().pruneNodeLocalProjection()?;
    service.spaceStore().importDeviceProfiles(observation.deviceProfiles)?;
    service.spaceStore().importTopologyRecords(observation.topology)?;
    let space = service.spaceStore().observePairedDeviceSpace(peer.to_owned(), observation.space)?;
    toCoreValue(space).map_err(|error| error.to_string())
}

/// Compare complete immutable records on Core, independent of export ordering.
/// An ID match alone is insufficient, and duplicate IDs never prove equality.
#[cfg(any(test, feature = "full"))]
pub(crate) fn sameControlOperations(left: &[SyncOperation], right: &[SyncOperation]) -> bool {
    use std::collections::BTreeMap;
    let leftById: BTreeMap<_, _> = left.iter().map(|operation| (&operation.opId, operation)).collect();
    let rightById: BTreeMap<_, _> = right.iter().map(|operation| (&operation.opId, operation)).collect();
    leftById.len() == left.len() && rightById.len() == right.len() && leftById == rightById
}

/// Legacy callers still get the original complete snapshot if it fits. The
/// preflight holds at most one decoded authority record before building it.
pub(crate) fn requireLegacySnapshotFits(service: &dyn NodeSpaceContext, peer: &str) -> Result<(), String> {
    if !service.isSpaceEndpoint() { return Ok(()); }
    let header = header(service, peer)?;
    let metadata = encodedLinkSize(&header).map_err(|error| error.to_string())?;
    requireControlFits(service, &header.space.spaceId, metadata)
}
pub(crate) fn requireLegacyControlFits(service: &dyn NodeSpaceContext) -> Result<(), String> {
    if !service.isSpaceEndpoint() { return Ok(()); }
    requireControlFits(service, &service.spaceStore().space()?.spaceId, 0)
}
fn requireControlFits(service: &dyn NodeSpaceContext, space: &str, metadata: usize) -> Result<(), String> {
    let mut page = service.networkControlStore().spaceControlExport(space, None, None)?;
    let revision = page.revision.clone();
    let maximum = service.peers()?.maxMessageBytes();
    let mut size = metadata.saturating_add(ENVELOPE_BYTES);
    while let Some(operation) = page.operation {
        size = size.saturating_add(encodedLinkSize(&operation).map_err(|error| error.to_string())?);
        if size > maximum {
            return Err("SPACE_MESSAGE_LIMIT: Complete authority exceeds the endpoint limit; bounded export is required".into());
        }
        page = service.networkControlStore().spaceControlExport(space, Some(&revision), Some(&operation.opId))?;
    }
    Ok(())
}

/// Collects a complete correlated authority export before any local import.
/// Untrusted counts and cursors cannot authorize partial or mixed snapshots.
pub(crate) struct SnapshotCollector {
    header: SnapshotHeader,
    operations: Vec<SyncOperation>,
    complete: bool,
}
impl SnapshotCollector {
    /// Validates a manifest before allocating any operation collection.
    pub fn new(header: SnapshotHeader) -> Result<Self, String> {
        if header.controlCount > 4096 || header.controlRevision.len() != 64
            || !header.controlRevision.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return Err("SPACE_EXPORT_INVALID: Invalid Space export manifest".into());
        }
        Ok(Self { header, operations: Vec::new(), complete: false })
    }
    /// Builds the next cursor from the last accepted immutable operation.
    pub fn request(&self) -> ControlPageRequest {
        ControlPageRequest { spaceId: self.header.space.spaceId.clone(), revision: self.header.controlRevision.clone(),
            after: self.operations.last().map(|operation| operation.opId.clone()) }
    }
    /// Validates page correlation and strictly increasing immutable operation identities.
    pub fn push(&mut self, page: SpaceControlExport) -> Result<bool, String> {
        if self.complete { return Err("SPACE_EXPORT_INVALID: Export is already complete".into()); }
        if page.revision != self.header.controlRevision || page.count != self.header.controlCount {
            return Err("SPACE_EXPORT_CHANGED: Space control projection changed".into());
        }
        let Some(operation) = page.operation else {
            if self.operations.len() != self.header.controlCount { return Err("SPACE_EXPORT_INVALID: Incomplete control export".into()); }
            self.complete = true;
            return Ok(true);
        };
        if self.operations.len() >= self.header.controlCount
            || self.operations.last().is_some_and(|old| old.opId >= operation.opId) {
            return Err("SPACE_EXPORT_INVALID: Invalid operation cursor".into());
        }
        self.operations.push(operation);
        Ok(false)
    }
    /// Verifies immutable authority and attaches the latest presentation projection.
    pub fn finish(self, current: SnapshotHeader) -> Result<PeerSpaceSnapshot, String> {
        if !self.complete || self.operations.len() != self.header.controlCount
            || self.header.space != current.space
            || self.header.controlRevision != current.controlRevision
            || self.header.controlCount != current.controlCount
            || self.header.departures != current.departures {
            return Err("SPACE_EXPORT_CHANGED: Space authority changed".into());
        }
        Ok(current.withOperations(self.operations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_link::CoreLinkError;
    fn manifest(count: usize) -> SnapshotHeader {
        SnapshotHeader { space: CoreSpace { spaceId: "space".into(), spaceName: "Name".into(),
            spaceRevision: 1, members: vec!["edge".into(), "core".into()] },
            deviceProfiles: Vec::new(), topology: Vec::new(), departures: Vec::new(),
            controlRevision: "a".repeat(64), controlCount: count }
    }
    fn operation(id: &str) -> SyncOperation {
        SyncOperation { opId: id.into(), originDeviceId: "edge".into(), sequence: 1,
            domain: "network_control".into(), entityType: "command".into(), entityId: id.into(),
            operation: "apply".into(), semantics: operit_store::SyncOperationStore::SyncOperationSemantics::Transaction,
            payload: serde_json::json!({"unknown": ["设备", 7]}), createdAt: 1, schemaVersion: 1 }
    }
    /// Creates a page correlated to the fixture manifest.
    fn page(count: usize, id: Option<&str>) -> SpaceControlExport {
        SpaceControlExport { revision: "a".repeat(64), count, operation: id.map(operation) }
    }
    #[test]
    fn projection_reference_requires_exact_unique_immutable_operations() {
        let a = operation("a");
        let b = operation("b");
        assert!(sameControlOperations(&[a.clone(), b.clone()], &[b.clone(), a.clone()]));
        assert!(!sameControlOperations(&[a.clone()], &[a.clone(), b.clone()]));
        assert!(!sameControlOperations(&[a.clone(), a.clone()], &[a.clone(), a.clone()]));
        let mut changed = a.clone();
        changed.payload["future"] = serde_json::json!({"mustPreserve": true});
        assert!(!sameControlOperations(&[a.clone()], &[changed]));
        let mut changed = a.clone();
        changed.createdAt += 1;
        assert!(!sameControlOperations(&[a], &[changed]));
    }
    #[test]
    fn old_protocol_omission_does_not_enable_projection_references() {
        let old = serde_json::json!({"version":1,"nodeId":"peer","endpoint":true,
            "authorityMode":"localReplay","maxMessageBytes":8192,"snapshotPaging":true});
        let mut protocol: ProjectionProtocol = serde_json::from_value(old).unwrap();
        assert!(!protocol.projectionReference);
        protocol.validate("peer").unwrap();
        protocol.projectionReference = true;
        protocol.snapshotPaging = false;
        protocol.endpoint = false;
        assert!(protocol.validate("peer").is_err());
    }
    #[test]
    fn negotiated_protocol_never_downgrades_on_failure_or_advertises_hosted_authority() {
        assert!(decodeProtocolResponse("peer", Err(CoreLinkError::internal("Unknown node Space method"))).is_err());
        for code in ["PEER_SECURITY", "METHOD_NOT_FOUND", "SPACE_ID_MISMATCH", "INTERNAL"] {
            assert!(decodeProtocolResponse("peer", Err(CoreLinkError::new(code, "failure"))).is_err());
        }
        assert!(decodeProtocolResponse("peer", Ok(CoreValue::Null)).is_err());
        let mut protocol = ProjectionProtocol { version: VERSION, nodeId: "peer".into(), endpoint: true,
            authorityMode: "localReplay".into(), maxMessageBytes: 8192, snapshotPaging: true, projectionReference: true };
        assert!(protocol.validate("peer").is_ok());
        protocol.endpoint = false;
        protocol.projectionReference = false;
        assert!(protocol.validate("peer").is_ok(), "Core exposes the same paged authority contract");
        assert!(protocol.validate("other").is_err());
        protocol.version += 1;
        assert!(protocol.validate("peer").is_err());
        protocol.version = VERSION;
        protocol.authorityMode = "coreManaged".into();
        assert!(protocol.validate("peer").is_err());
        protocol.authorityMode = "localReplay".into();
        protocol.maxMessageBytes = ENVELOPE_BYTES;
        assert!(protocol.validate("peer").is_err());
    }
    #[test]
    fn bounded_export_requires_eof_and_unchanged_entire_manifest() {
        let header = manifest(2);
        let mut collector = SnapshotCollector::new(header.clone()).unwrap();
        assert_eq!(collector.request().after, None);
        assert!(!collector.push(page(2, Some("a"))).unwrap());
        assert_eq!(collector.request().after.as_deref(), Some("a"));
        assert!(!collector.push(page(2, Some("b"))).unwrap());
        assert!(collector.finish(header.clone()).is_err());
        let mut collector = SnapshotCollector::new(header.clone()).unwrap();
        collector.push(page(2, Some("a"))).unwrap();
        collector.push(page(2, Some("b"))).unwrap();
        assert!(collector.push(page(2, None)).unwrap());
        let snapshot = collector.finish(header.clone()).unwrap();
        assert_eq!(snapshot.controlOperations, vec![operation("a"), operation("b")]);
        let mut collector = SnapshotCollector::new(manifest(0)).unwrap();
        collector.push(page(0, None)).unwrap();
        let mut changed = manifest(0);
        changed.space.spaceRevision += 1;
        assert!(collector.finish(changed).is_err());
    }
    /// Presentation changes cannot invalidate stable authority or supply authority themselves.
    #[test]
    fn bounded_export_uses_current_presentation_under_unchanged_authority() {
        let mut collector = SnapshotCollector::new(manifest(0)).unwrap();
        collector.push(page(0, None)).unwrap();
        let mut current = manifest(0);
        current.deviceProfiles.push(CoreSpaceDeviceProfile {
            nodeId: "core".into(), displayName: "Updated Core".into(), userName: "User".into(),
            platform: "test".into(), model: "test".into(), coreVersion: None, pairingIncarnation: None, updatedAt: 2,
        });
        let snapshot = collector.finish(current.clone()).unwrap();
        assert_eq!(snapshot.deviceProfiles, current.deviceProfiles);
        let mut collector = SnapshotCollector::new(manifest(0)).unwrap();
        collector.push(page(0, None)).unwrap();
        current.departures.push(operation("departure"));
        assert!(collector.finish(current).is_err());
    }
    #[test]
    fn bounded_export_rejects_missing_repeated_excess_or_changed_records() {
        let mut collector = SnapshotCollector::new(manifest(2)).unwrap();
        assert!(collector.push(page(2, None)).is_err());
        collector.push(page(2, Some("b"))).unwrap();
        assert!(collector.push(page(2, Some("a"))).is_err());
        assert!(collector.push(page(2, Some("b"))).is_err());
        assert!(collector.push(page(3, Some("c"))).is_err());
        let mut changed = page(2, Some("c"));
        changed.revision = "b".repeat(64);
        assert!(collector.push(changed).is_err());
        let mut collector = SnapshotCollector::new(manifest(0)).unwrap();
        assert!(collector.push(page(0, Some("a"))).is_err());
        assert!(collector.push(page(0, None)).unwrap());
        assert!(collector.push(page(0, None)).is_err());
        assert!(collector.finish(manifest(0)).is_ok());
        assert!(SnapshotCollector::new(manifest(4097)).is_err());
    }
    #[test]
    fn routed_budget_includes_variable_identity_lengths_and_actual_wire_envelope() {
        use operit_link::{CoreCallRequest, CoreLinkRequest, RoutedCoreRequest, RoutedCoreRequestKind};
        let request = CoreCallRequest::new("test", "node.space", "observeSpaceSnapshot", CoreValue::String("payload".into()));
        let space = "空间".repeat(160);
        let local = "本机".repeat(160);
        let peer = "对端".repeat(160);
        let routed = RoutedCoreRequest { spaceId: space.clone(), originNodeId: local.clone(), targetNodeId: peer.clone(),
            ttl: 0, routeKind: RoutedCoreRequestKind::Target, payload: request.clone() };
        let wire = CoreCallRequest::new(request.requestId.0.clone(), request.target.clone(), request.methodName.clone(), toCoreValue(routed).unwrap());
        let length = operit_link::encodeLink(CoreLinkRequest::Call(wire)).unwrap().len();
        assert!(requireRoutedRequestFits(&request, &space, &local, &peer, 0, length + ENVELOPE_BYTES).is_ok());
        assert!(requireRoutedRequestFits(&request, &space, &local, &peer, 0, length + ENVELOPE_BYTES - 1).is_err());
    }
    #[test]
    fn payload_budget_matches_wire_encoding_and_reserves_envelope() {
        for value in [CoreValue::Null, toCoreValue(operation("a")).unwrap(),
            CoreValue::Bytes(vec![128; 4096].into())] {
            let bytes = operit_link::encodeLink(&value).unwrap();
            assert_eq!(encodedLinkSize(&value).unwrap(), bytes.len());
            assert!(requirePayloadFits(&value, bytes.len() + ENVELOPE_BYTES).is_ok());
            assert!(requirePayloadFits(&value, bytes.len() + ENVELOPE_BYTES - 1).is_err());
        }
        assert!(requireEncodedFits(&[255; 8192], 8192).is_err());
    }
}
