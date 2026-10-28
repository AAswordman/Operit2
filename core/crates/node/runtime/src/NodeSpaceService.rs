//! Space admission control plane, independent of Chat/Provider/Tool execution.
//! Both Core and Edge run peer/space_join.rs; only routing and post-join work differ.
#![allow(non_snake_case)]
use operit_host_api::{RuntimeStorageHost, TimeUtils::currentTimeMillis};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse, CoreValue, CoreLinkError, RoutedCoreRequest, RoutedCoreRequestKind};
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceDeviceProfile, CoreSpaceStore, CoreSpaceTopologyRecord};
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::NetworkControlStore::NetworkControlStore;
use operit_store::SyncOperationStore::SyncOperation;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::RuntimePeerService::RuntimePeerService;

/// Runtime 的 Space 业务对象；仍使用标准 Link Call，不新增握手消息或 HTTP 路径。
pub const NODE_SPACE_TARGET: &str = "node.space";
// Same-Space, authenticated routing only; never available to an unadmitted applicant.
pub const NODE_SPACE_APPROVAL_TARGET: &str = "node.space.approval";
/// Authority exchange remains available to admitted endpoints without storage grants.
pub const NODE_SPACE_CONTROL_TARGET: &str = "node.space.control";

/// Expected protocol failures must fit the bounded peer's error response too.
/// Keep native diagnostics for unexpected failures, without attaching a large
/// host backtrace to message-limit/version/export retry errors.
pub fn spaceCallError(message: String) -> CoreLinkError {
    let code = message.split(':').next().unwrap_or("");
    if matches!(code, "SPACE_MESSAGE_LIMIT" | "SPACE_PROTOCOL_INCOMPATIBLE"
        | "SPACE_EXPORT_CHANGED" | "SPACE_EXPORT_CURSOR_INVALID"
        | "SPACE_EXPORT_MIGRATION_REQUIRED" | "SPACE_EXPORT_INVALID") {
        CoreLinkError::new(code.to_owned(), message)
    } else { CoreLinkError::internal(message) }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PeerSpaceSnapshot {
    pub(crate) space: CoreSpace,
    pub(crate) deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    pub(crate) controlOperations: Vec<SyncOperation>,
    pub(crate) topology: Vec<CoreSpaceTopologyRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) departures: Vec<SyncOperation>,
}
pub(crate) type PeerSpaceJoin = PeerSpaceSnapshot;

/// A standard wire sequence whose records are released after each element is
/// encoded. These replies serialize once and contain no live stream handles.
struct OwnedSequence<T>(std::cell::RefCell<Option<Vec<T>>>);
impl<T> OwnedSequence<T> {
    fn new(values: Vec<T>) -> Self { Self(std::cell::RefCell::new(Some(values))) }
    fn isEmpty(&self) -> bool { self.0.borrow().as_ref().is_none_or(Vec::is_empty) }
}
impl<T: Serialize> Serialize for OwnedSequence<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let values = self.0.borrow_mut().take()
            .ok_or_else(|| serde::ser::Error::custom("Space reply records were already encoded"))?;
        let mut sequence = serializer.serialize_seq(Some(values.len()))?;
        for value in values { sequence.serialize_element(&value)?; }
        sequence.end()
    }
}

#[derive(Serialize)]
struct OwnedSnapshotReply {
    space: CoreSpace,
    deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    controlOperations: OwnedSequence<SyncOperation>,
    topology: Vec<CoreSpaceTopologyRecord>,
    #[serde(skip_serializing_if = "OwnedSequence::isEmpty")]
    departures: OwnedSequence<SyncOperation>,
}
impl From<PeerSpaceSnapshot> for OwnedSnapshotReply {
    fn from(snapshot: PeerSpaceSnapshot) -> Self {
        Self { space: snapshot.space, deviceProfiles: snapshot.deviceProfiles,
            controlOperations: OwnedSequence::new(snapshot.controlOperations), topology: snapshot.topology,
            departures: OwnedSequence::new(snapshot.departures) }
    }
}

/// Consume JSON payloads progressively instead of retaining two complete trees.
fn jsonIntoCoreValue(value: serde_json::Value) -> Result<CoreValue, String> {
    use serde_json::Value;
    Ok(match value {
        Value::Null => CoreValue::Null,
        Value::Bool(value) => CoreValue::Bool(value),
        Value::Number(value) => toCoreValue(value).map_err(|error| error.to_string())?,
        Value::String(value) => CoreValue::String(value),
        Value::Array(values) => CoreValue::List(values.into_iter().map(jsonIntoCoreValue).collect::<Result<_, _>>()?),
        Value::Object(values) => CoreValue::Map(values.into_iter()
            .map(|(key, value)| jsonIntoCoreValue(value).map(|value| (key, value))).collect::<Result<_, _>>()?),
    })
}

/// Move each large JSON payload into the wire tree before converting the next.
fn operationsIntoCoreValue(operations: Vec<SyncOperation>) -> Result<CoreValue, String> {
    operations.into_iter().map(|mut operation| {
        let payload = std::mem::take(&mut operation.payload);
        let CoreValue::Map(mut fields) = toCoreValue(operation).map_err(|error| error.to_string())? else {
            return Err("Invalid control operation wire envelope".into());
        };
        fields.insert("payload".into(), jsonIntoCoreValue(payload)?);
        Ok(CoreValue::Map(fields))
    }).collect::<Result<Vec<_>, String>>().map(CoreValue::List)
}
fn takeSnapshotOperations(snapshot: &mut PeerSpaceSnapshot) -> (Vec<SyncOperation>, Vec<SyncOperation>) {
    (std::mem::take(&mut snapshot.controlOperations), std::mem::take(&mut snapshot.departures))
}
fn attachSnapshotOperations(value: &mut CoreValue, operations: (Vec<SyncOperation>, Vec<SyncOperation>)) -> Result<(), String> {
    let CoreValue::Map(fields) = value else { return Err("Invalid Space snapshot wire envelope".into()); };
    fields.insert("controlOperations".into(), operationsIntoCoreValue(operations.0)?);
    if !operations.1.is_empty() { fields.insert("departures".into(), operationsIntoCoreValue(operations.1)?); }
    Ok(())
}
impl PeerSpaceSnapshot {
    fn intoCoreValue(mut self) -> Result<CoreValue, String> {
        let operations = takeSnapshotOperations(&mut self);
        let mut value = toCoreValue(self).map_err(|error| error.to_string())?;
        attachSnapshotOperations(&mut value, operations)?;
        Ok(value)
    }
}

fn jsonField<'a>(value: &'a mut serde_json::Value, name: &str, index: usize) -> Option<&'a mut serde_json::Value> {
    match value { serde_json::Value::Object(fields) => fields.get_mut(name),
        serde_json::Value::Array(fields) => fields.get_mut(index), _ => None }
}

fn operationPayloads(value: Option<&mut serde_json::Value>) -> Result<Vec<serde_json::Value>, String> {
    let Some(value) = value else { return Ok(Vec::new()); };
    let operations = value.as_array_mut().ok_or("Invalid snapshot operations")?;
    operations.iter_mut().map(|operation| {
        jsonField(operation, "payload", 8).map(serde_json::Value::take).ok_or_else(|| "Missing operation payload".into())
    }).collect()
}
#[cfg(test)]
fn operationsFromJson(mut value: serde_json::Value) -> Result<Vec<SyncOperation>, String> {
    let payloads = operationPayloads(Some(&mut value))?;
    let mut operations: Vec<SyncOperation> = serde_json::from_value(value).map_err(|error| error.to_string())?;
    for (operation, payload) in operations.iter_mut().zip(payloads) { operation.payload = payload; }
    Ok(operations)
}

// Decode only business arguments after the authenticated route has been scanned.
// Header frames never remain on the task stack while decoding authority payloads.
struct DecodedSpaceArgs {
    metadata: serde_json::Value,
    operations: Vec<SyncOperation>,
    departures: Vec<SyncOperation>,
}
struct EncodedSpaceArgs<const KIND: u8>(DecodedSpaceArgs);
// KIND: snapshot = 0, control array = 1, bounded control = 2.
struct SpaceArgsSeed { kind: u8 }
struct OperationList(Vec<SyncOperation>);
impl<'de> Deserialize<'de> for OperationList {
    /// Decodes the declared business value without reconstructing routing headers.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OperationsVisitor;
        impl<'de> serde::de::Visitor<'de> for OperationsVisitor {
            type Value = OperationList;
            /// Describes the required authority sequence.
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("authority operations") }
            /// Builds only typed operations while preserving their original payload values.
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut list: A) -> Result<Self::Value, A::Error> {
                let mut operations = Vec::new();
                while let Some(operation) = list.next_element::<SyncOperation>()? {
                    operations.try_reserve_exact(1).map_err(serde::de::Error::custom)?;
                    operations.push(operation);
                }
                Ok(OperationList(operations))
            }
        }
        deserializer.deserialize_seq(OperationsVisitor)
    }
}
impl<'de> serde::de::DeserializeSeed<'de> for SpaceArgsSeed {
    type Value = DecodedSpaceArgs;
    /// Decodes the declared business value without reconstructing routing headers.
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        if self.kind == 1 {
            return Ok(DecodedSpaceArgs { metadata: serde_json::Value::Null,
                operations: OperationList::deserialize(deserializer)?.0, departures: Vec::new() });
        }
        deserializer.deserialize_any(self)
    }
}
impl SpaceArgsSeed {
    /// Matches the authority field declared by this command kind.
    fn operationField(&self, key: &str) -> bool {
        key == if self.kind == 0 { "controlOperations" } else { "operations" }
    }
}
impl<'de> serde::de::Visitor<'de> for SpaceArgsSeed {
    type Value = DecodedSpaceArgs;
    /// Describes the required business argument container.
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("routed Call arguments") }
    /// Separates named metadata from typed authority records.
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut fields: A) -> Result<Self::Value, A::Error> {
        let mut metadata = serde_json::Map::new();
        let (mut operations, mut departures) = (None, None);
        while let Some(key) = fields.next_key::<String>()? {
            if self.operationField(&key) || (self.kind == 0 && key == "departures") {
                let slot = if key == "departures" { &mut departures } else { &mut operations };
                if slot.is_some() { return Err(serde::de::Error::custom("Duplicate authority field")); }
                *slot = Some(fields.next_value::<OperationList>()?.0);
                metadata.insert(key, serde_json::Value::Array(Vec::new()));
            } else { metadata.insert(key, fields.next_value()?); }
        }
        Ok(DecodedSpaceArgs { metadata: serde_json::Value::Object(metadata),
            operations: operations.ok_or_else(|| serde::de::Error::custom("Missing authority operations"))?,
            departures: departures.unwrap_or_default() })
    }
    /// Separates positional metadata from typed authority records.
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut fields: A) -> Result<Self::Value, A::Error> {
        let mut metadata = Vec::new();
        let (mut operations, mut departures) = (None, None);
        for index in 0..if self.kind == 0 { 5 } else { 3 } {
            if index == 2 || (self.kind == 0 && index == 4) {
                let list = fields.next_element::<OperationList>()?;
                if index == 4 { departures = list.map(|list| list.0); }
                else { operations = list.map(|list| list.0); }
                metadata.push(serde_json::Value::Array(Vec::new()));
            } else {
                metadata.push(fields.next_element()?.ok_or_else(|| serde::de::Error::custom("Missing argument field"))?);
            }
        }
        if fields.next_element::<serde::de::IgnoredAny>()?.is_some() { return Err(serde::de::Error::custom("Extra argument field")); }
        Ok(DecodedSpaceArgs { metadata: serde_json::Value::Array(metadata),
            operations: operations.ok_or_else(|| serde::de::Error::custom("Missing authority operations"))?,
            departures: departures.unwrap_or_default() })
    }
}
impl<'de, const KIND: u8> Deserialize<'de> for EncodedSpaceArgs<KIND> {
    /// Decodes the declared business value without reconstructing routing headers.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::DeserializeSeed;
        SpaceArgsSeed { kind: KIND }.deserialize(deserializer).map(Self)
    }
}
/// Decodes a required selected field after releasing the enclosing format scanner.
fn decodeSpaceField<T: serde::de::DeserializeOwned>(bytes: &[u8], name: &str, index: usize) -> Result<T, String> {
    let field = operit_link::codec::linkStructField(bytes, name, index).map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Missing Space field: {name}"))?;
    operit_link::decodeLink(field).map_err(|error| error.to_string())
}

/// Selects the payload slice without retaining any routing deserializer frames.
fn encodedSpaceArguments(bytes: &[u8]) -> Result<&[u8], String> {
    let mut arguments = bytes;
    for (field, index) in [("body", 1), ("args", 3), ("payload", 5), ("args", 3)] {
        arguments = operit_link::codec::linkStructField(arguments, field, index)
            .map_err(|error| error.to_string())?.ok_or("Missing routed argument field")?;
    }
    Ok(arguments)
}

/// Scans authenticated routing fields before entering the business payload decoder.
fn takeEncodedSpaceArgs<const KIND: u8>(bytes: &mut Vec<u8>) -> Result<DecodedSpaceArgs, String> {
    let arguments = encodedSpaceArguments(bytes)?;
    let args: EncodedSpaceArgs<KIND> = operit_link::decodeLink(arguments).map_err(|error| error.to_string())?;
    drop(std::mem::take(bytes));
    Ok(args.0)
}

fn encodeTypedReply<T: Serialize>(id: &operit_link::CoreRequestId, result: Result<T, CoreLinkError>) -> Result<Vec<u8>, String> {
    #[derive(Serialize)]
    struct Response<'a, T> { requestId: &'a operit_link::CoreRequestId, result: Result<T, CoreLinkError> }
    #[derive(Serialize)]
    #[serde(tag = "type", content = "body")]
    enum Envelope<'a, T> { Call(Response<'a, T>) }
    operit_link::encodeLink(Envelope::Call(Response { requestId: id, result })).map_err(|error| error.to_string())
}

/// Deserialize only operation metadata; their already-decoded JSON payloads
/// move unchanged into the result instead of rebuilding every tree node.
fn snapshotFromJson(mut value: serde_json::Value) -> Result<PeerSpaceSnapshot, String> {
    let control = operationPayloads(jsonField(&mut value, "controlOperations", 2))?;
    let departures = operationPayloads(jsonField(&mut value, "departures", 4))?;
    let mut snapshot: PeerSpaceSnapshot = serde_json::from_value(value).map_err(|error| error.to_string())?;
    for (operation, payload) in snapshot.controlOperations.iter_mut().zip(control) { operation.payload = payload; }
    for (operation, payload) in snapshot.departures.iter_mut().zip(departures) { operation.payload = payload; }
    Ok(snapshot)
}

/// Typed view of the existing Call envelope used to avoid an intermediate args tree.
#[derive(Deserialize)]
struct EncodedCall<A> { requestId: operit_link::CoreRequestId, target: String, methodName: String, args: A }
#[derive(Deserialize)]
enum EncodedCallTag { Call }

/// Peeks route metadata only; payloads are skipped without allocating a value tree.
pub fn encodedSpaceCallHeader(bytes: &[u8]) -> Option<(operit_link::CoreRequestId, RoutedCoreRequest<CoreCallRequest>)> {
    let _: EncodedCallTag = decodeSpaceField(bytes, "type", 0).ok()?;
    let body = operit_link::codec::linkStructField(bytes, "body", 1).ok()??;
    let outer: EncodedCall<RoutedCoreRequest<EncodedCall<serde::de::IgnoredAny>>> = operit_link::decodeLink(body).ok()?;
    let routed = outer.args;
    Some((outer.requestId, RoutedCoreRequest {
        spaceId: routed.spaceId, originNodeId: routed.originNodeId, targetNodeId: routed.targetNodeId,
        ttl: routed.ttl, routeKind: routed.routeKind,
        payload: CoreCallRequest { requestId: routed.payload.requestId, target: routed.payload.target,
            methodName: routed.payload.methodName, args: CoreValue::Null },
    }))
}

/// Join approval is separate from pairing and from synchronized membership.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpaceJoinStatus { Pending, Approving, Approved, Rejected, Cancelled, Expired, Joined }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceJoinRequest {
    pub requestId: String,
    pub targetDeviceId: String,
    pub applicantDeviceId: String,
    pub applicantName: String,
    pub spaceName: String,
    pub status: SpaceJoinStatus,
    pub createdAt: i64,
    pub expiresAt: i64,
    pub canApprove: bool,
    #[serde(default)]
    pub reviewerDeviceId: Option<String>,
    #[serde(default)]
    pub reviewerName: Option<String>,
    #[serde(default)]
    pub reviewerHops: Option<u32>,
    #[serde(default)]
    pub assignmentVersion: u64,
    #[serde(default)]
    pub decisionApprove: Option<bool>,
}


#[path = "peer/space_projection.rs"]
pub(crate) mod space_projection;
#[path = "peer/space_departure.rs"]
pub(crate) mod space_departure;
#[path = "peer/space_join.rs"]
pub(crate) mod space_join;
#[path = "peer/space_reconcile.rs"]
pub(crate) mod space_reconcile;
#[path = "peer/space_doctor.rs"]
pub(crate) mod space_doctor;
#[path = "peer/space_topology.rs"]
pub(crate) mod space_topology;

/// Hooks provided by the existing node runtime, not a second wire protocol.
#[async_trait::async_trait(?Send)]
pub trait NodeSpaceContext: Send + Sync {
    fn storage(&self) -> Arc<dyn RuntimeStorageHost>;
    fn spaceStore(&self) -> &CoreSpaceStore;
    fn networkControlStore(&self) -> &NetworkControlStore;
    fn peers(&self) -> Result<Arc<dyn RuntimePeerService>, String>;
    fn localNodeId(&self) -> String;
    fn isSpaceEndpoint(&self) -> bool { false }
    fn nodeIsReachable(&self, node: &str) -> Result<bool, String>;
    async fn callNode(&self, node: String, request: CoreCallRequest) -> CoreCallResponse;
    async fn afterJoined(&self, _peer: String) -> Result<(), String> { Ok(()) }
}

/// Shared Space protocol dispatch. Each receiver checks its authenticated direct peer first.
pub(crate) fn dispatchPeerSpaceCall(
    service: &dyn NodeSpaceContext, peer: &str, request: CoreCallRequest,
) -> Result<CoreValue, String> {
    if service.isSpaceEndpoint() {
        space_projection::requirePayloadFits(&request, service.peers()?.maxMessageBytes())?;
    }
    match request.methodName.as_str() {
        "projectionProtocol" => space_projection::negotiate(service, request.args),
        "snapshotHeader" => toCoreValue(space_projection::header(service, peer)?).map_err(|error| error.to_string()),
        "snapshotControlPage" => space_projection::controlPage(service, request.args),
        "observeSpaceProjection" => space_projection::observeProjection(service, peer, request.args),
        "snapshot" => peerSpaceSnapshotForPeer(service, peer)?.intoCoreValue(),
        "deviceSpace" => toCoreValue(service.spaceStore().initialize()?).map_err(|e| e.to_string()),
        "observeSpaceSnapshot" => {
            let snapshot: PeerSpaceSnapshot = fromCoreValue(request.args).map_err(|e| e.to_string())?;
            toCoreValue(observePeerSpaceSnapshot(service, peer, snapshot)?).map_err(|e| e.to_string())
        }
        "observePairedDeviceSpace" => {
            let space: CoreSpace = fromCoreValue(request.args).map_err(|e| e.to_string())?;
            toCoreValue(service.spaceStore().observePairedDeviceSpace(peer.to_string(), space)?).map_err(|e| e.to_string())
        }
        "requestJoin" | "joinStatus" | "cancelJoin" => space_join::receive(service, peer, request),
        "reconcileSharedSpace" => space_reconcile::receive(service, peer, request),
        "join" => Err("SPACE_JOIN_APPROVAL_REQUIRED: Submit a join request for local approval first".into()),
        _ => Err("Unknown node Space method".into()),
    }
}

pub(crate) fn peerSpaceSnapshot(service: &dyn NodeSpaceContext) -> Result<PeerSpaceSnapshot, String> {
    let space = service.spaceStore().initialize()?;
    peerSpaceSnapshotFor(service, space)
}
pub(crate) fn peerSpaceSnapshotFor(service: &dyn NodeSpaceContext, space: CoreSpace) -> Result<PeerSpaceSnapshot, String> {
    // Presentation history is local to the store. A Space snapshot carries only
    // the members of this projection, not devices retained from earlier Spaces.
    let deviceProfiles = service.spaceStore().deviceProfiles()?.into_iter()
        .filter(|(node, _)| space.members.contains(node)).map(|(_, profile)| profile).collect();
    let topology = service.spaceStore().topologyRecords()?.into_iter()
        .filter(|(node, _)| space.members.contains(node)).map(|(_, record)| record).collect();
    let snapshot = PeerSpaceSnapshot {
        space, deviceProfiles,
        controlOperations: service.networkControlStore().currentSpaceOperations()?,
        topology, departures: Vec::new(),
    };
    CoreSpaceStore::validateSpaceProfiles(&snapshot.space, &snapshot.deviceProfiles)?;
    Ok(snapshot)
}

/// Only direct former neighbors receive this node's pending exit receipts.
pub(crate) fn peerSpaceSnapshotForPeer(service: &dyn NodeSpaceContext, peer: &str) -> Result<PeerSpaceSnapshot, String> {
    space_projection::requireLegacySnapshotFits(service, peer)?;
    let mut snapshot = peerSpaceSnapshot(service)?;
    snapshot.departures = space_departure::pending(service, peer)?;
    Ok(snapshot)
}

pub(crate) fn observePeerSpaceSnapshot(service: &dyn NodeSpaceContext, peerNodeId: &str, snapshot: PeerSpaceSnapshot) -> Result<CoreSpace, String> {
        CoreSpaceStore::validateSpaceProfiles(&snapshot.space, &snapshot.deviceProfiles)?;
        if !snapshot.space.members.iter().any(|node| node == peerNodeId) {
            return Err("Paired device is not present in its announced device space".into());
        }
        space_departure::observe(service, peerNodeId, &snapshot.departures, &snapshot.controlOperations)?;
        // Only the local membership is needed here. Rebuilding its full
        // authority snapshot alongside the received snapshot doubles peak RAM.
        let local = service.spaceStore().initialize()?;
        let crossing = local.spaceId != snapshot.space.spaceId;
        if crossing {
            let localOperations = service.networkControlStore().currentSpaceOperations()?;
            let localAdmittedSource = localOperations.iter().map(|operation| {
                serde_json::from_value::<operit_store::NetworkControlStore::NetworkControlCommandRecord>(operation.payload.clone())
                    .map_err(|error| error.to_string())
            }).collect::<Result<Vec<_>, _>>()?.iter().any(|record| {
                record.spaceId == local.spaceId && matches!(&record.command,
                    operit_store::NetworkControlStore::NetworkControlCommand::AdmitSpace { sourceSpaceId, .. }
                    if sourceSpaceId == &snapshot.space.spaceId)
            });
            if localAdmittedSource {
                service.networkControlStore().validateSpaceAdmission(&local.spaceId, &snapshot.space.spaceId,
                    &snapshot.space.members.iter().cloned().collect(), &localOperations)?;
                // This endpoint has already migrated; publish its target projection to the source peer.
                service.spaceStore().importDeviceProfiles(snapshot.deviceProfiles)?;
                return Ok(local);
            }
            if !snapshot.space.members.iter().any(|node| node == &service.localNodeId()) {
                // Independent paired Spaces do not merge without a target-side admission.
                return Ok(local);
            }
            if !local.members.iter().any(|node| node == peerNodeId) {
                return Err("Space migration must arrive through an existing source Space member".into());
            }
            service.networkControlStore().validateSpaceAdmission(&snapshot.space.spaceId,
                &local.spaceId, &local.members.iter().cloned().collect(), &snapshot.controlOperations)?;
        }
        let policy = service.networkControlStore().validateSpacePolicy(&snapshot.space.spaceId, &snapshot.controlOperations)?;
        for node in &snapshot.space.members {
            if !policy.memberNodeIds.contains(node) {
                return Err(format!("Space snapshot contains an unauthorized member: {node}"));
            }
        }
        drop(policy);
        // A validated endpoint batch must not retain every expanded JSON payload
        // while NVS opens the existing authority record. Compact progressively,
        // then decode only the operation currently being durably applied. Keep
        // full validation before the first publication and preserve unknown data.
        let compact = service.isSpaceEndpoint() && !service.storage().retainPreferencesCache();
        let (operations, encoded) = if compact {
            let mut encoded = Vec::new();
            encoded.try_reserve_exact(snapshot.controlOperations.len()).map_err(|error| error.to_string())?;
            for operation in snapshot.controlOperations {
                encoded.push(serde_json::to_string(&operation).map_err(|error| error.to_string())?);
            }
            (Vec::new(), encoded)
        } else { (snapshot.controlOperations, Vec::new()) };
        // Departure acknowledgements have already been processed above.
        drop(snapshot.departures);
        // Only non-replicating endpoints discard obsolete cached projections;
        // authorization validation above still precedes every imported fact.
        service.spaceStore().pruneNodeLocalProjection()?;
        service.networkControlStore().pruneNodeLocalProjection()?;
        // Profiles and authorization must be available before any member reference is published.
        service.spaceStore().importDeviceProfiles(snapshot.deviceProfiles)?;
        service.spaceStore().importTopologyRecords(snapshot.topology)?;
        // Release each accepted payload before reading/publishing the next
        // durable record; validation above has already checked the full batch.
        for operation in operations {
            service.networkControlStore().applyBootstrapOperation(&operation)?;
        }
        for json in encoded {
            let operation = serde_json::from_str(&json).map_err(|error| error.to_string())?;
            drop(json);
            service.networkControlStore().applyBootstrapOperation(&operation)?;
        }
        if crossing {
            service.spaceStore().adopt(snapshot.space)?;
        } else {
            service.spaceStore().observePairedDeviceSpace(peerNodeId.to_string(), snapshot.space)?;
        }
        // All received control payloads have been released before this replay.
        space_join::applyRemovedMemberLocalEffects(service, &service.networkControlStore().currentSpaceOperations()?)?;
        service.spaceStore().space()
    }

/// Lightweight assembly for devices without the application execution runtime.
pub struct NodeSpaceService {
    storage: Arc<dyn RuntimeStorageHost>,
    space: CoreSpaceStore,
    control: NetworkControlStore,
    peers: Arc<dyn RuntimePeerService>,
    localNodeId: String,
}
impl NodeSpaceService {
    pub fn new(storage: Arc<dyn RuntimeStorageHost>, peers: Arc<dyn RuntimePeerService>) -> Result<Arc<Self>, String> {
        let localNodeId = CoreNodeIdentityStore::new(storage.clone()).initialize()?.nodeId;
        Ok(Arc::new(Self {
            space: CoreSpaceStore::newNodeLocal(storage.clone()), control: NetworkControlStore::newNodeLocal(storage.clone())?,
            storage, peers, localNodeId,
        }))
    }
    /// Initialize the same membership and creator policy, but keep this endpoint
    /// projection node-local rather than registering a business data replica.
    pub fn initialize(&self, profile: CoreSpaceDeviceProfile) -> Result<(), String> {
        if profile.nodeId != self.localNodeId { return Err("Local profile identity mismatch".into()); }
        self.space.initialize()?;
        self.control.migrateNodeLocalProjection(self.storage.as_ref())?;
        self.space.pruneNodeLocalProjection()?;
        self.control.pruneNodeLocalProjection()?;
        let existing = self.space.deviceProfiles()?.remove(&self.localNodeId);
        if !existing.is_some_and(|old| old.displayName == profile.displayName && old.userName == profile.userName
            && old.platform == profile.platform && old.model == profile.model && old.coreVersion == profile.coreVersion) {
            self.space.importDeviceProfiles(vec![profile])?;
        }
        self.control.initializeCurrentSpace()?;
        // Authority can commit before a storage failure interrupts projection
        // cleanup. Finish its local effects without waiting for another peer
        // delivery, including when the removed peer is already offline.
        space_join::applyRemovedMemberLocalEffects(self, &self.control.currentSpaceOperations()?)?;
        Ok(())
    }
    fn requirePairedSpacePeer(&self, peer: &str) -> Result<(), String> {
        // Join rides any current pairing: the direction the pairing was
        // established in is transport history, not applicant authority.
        if !self.peers.hasPairing(peer, crate::NodeServices::PairingDirection::Either).map_err(|e| e.to_string())? {
            return Err("Node Space calls require a current direct pairing".into());
        }
        Ok(())
    }
    pub fn acceptPeerSpaceCall(&self, peer: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
        self.requirePairedSpacePeer(peer)?;
        dispatchPeerSpaceCall(self, peer, request)
    }
    /// Encodes the existing snapshot response directly from typed records, without
    /// expanding every JSON authority payload into a second Link value tree.
    pub fn encodePeerSpaceSnapshotReply(&self, peer: &str, id: &operit_link::CoreRequestId) -> Result<Vec<u8>, String> {
        let result = self.requirePairedSpacePeer(peer).and_then(|_| peerSpaceSnapshotForPeer(self, peer))
            .map_err(spaceCallError);
        encodeTypedReply(id, result.map(OwnedSnapshotReply::from))
    }
    /// Decodes applicant commands directly and returns the ordinary consent projection.
    pub fn encodePeerSpaceJoinReply(&self, peer: &str, bytes: &mut Vec<u8>, id: operit_link::CoreRequestId,
        method: &str) -> Result<Vec<u8>, String> {
        let result = space_projection::requireEncodedFits(bytes, self.peers.maxMessageBytes())
            .and_then(|()| space_join::receiveEncoded(self, peer, bytes, method)).map_err(spaceCallError);
        encodeTypedReply(&id, result)
    }

    /// The router has checked the immutable route header. Decode the same bytes
    /// directly into typed snapshot records and apply the ordinary observation.
    pub fn encodePeerSpaceObservationReply(&self, peer: &str, bytes: &mut Vec<u8>, id: operit_link::CoreRequestId) -> Result<Vec<u8>, String> {
        let result = (|| {
            self.requirePairedSpacePeer(peer)?;
            space_projection::requireEncodedFits(bytes, self.peers.maxMessageBytes())?;
            // Reuse the durable JSON record decoder for business arguments only;
            // routed headers are skipped without allocating a JSON value tree.
            let args = takeEncodedSpaceArgs::<0>(bytes)?;
            let mut snapshot = snapshotFromJson(args.metadata)?;
            snapshot.controlOperations = args.operations;
            snapshot.departures = args.departures;
            toCoreValue(observePeerSpaceSnapshot(self, peer, snapshot)?).map_err(|error| error.to_string())
        })().map_err(spaceCallError);
        operit_link::encodeLink(operit_link::CoreLinkResponse::Call(CoreCallResponse { requestId: id, result })).map_err(|error| error.to_string())
    }
    /// Use the ordinary control admission and publication with a compact input
    /// and a typed standard reply, without a second Link payload tree.
    pub fn encodePeerSpaceControlReply(&self, peer: &str, bytes: &mut Vec<u8>, id: operit_link::CoreRequestId) -> Result<Vec<u8>, String> {
        self.encodePeerSpaceControlReplyWithLimit(peer, bytes, id, false)
    }
    pub fn encodePeerSpaceBoundedControlReply(&self, peer: &str, bytes: &mut Vec<u8>, id: operit_link::CoreRequestId) -> Result<Vec<u8>, String> {
        self.encodePeerSpaceControlReplyWithLimit(peer, bytes, id, true)
    }
    fn encodePeerSpaceControlReplyWithLimit(&self, peer: &str, bytes: &mut Vec<u8>, id: operit_link::CoreRequestId, bounded: bool) -> Result<Vec<u8>, String> {
        let result = (|| -> Result<Vec<SyncOperation>, String> {
            space_projection::requireEncodedFits(bytes, self.peers.maxMessageBytes())?;
            requireSpaceControlSource(self, peer)?;
            let decoded = if bounded { takeEncodedSpaceArgs::<2>(bytes)? } else { takeEncodedSpaceArgs::<1>(bytes)? };
            let mut args = decoded.metadata;
            let maximum = if bounded {
                let version = jsonField(&mut args, "version", 0).and_then(|value| value.as_u64());
                let maximum = jsonField(&mut args, "maxMessageBytes", 1).and_then(|value| value.as_u64())
                    .and_then(|value| usize::try_from(value).ok()).unwrap_or(0);
                if version != Some(u64::from(space_projection::VERSION)) || maximum <= space_projection::ENVELOPE_BYTES {
                    return Err("SPACE_PROTOCOL_INCOMPATIBLE: Unsupported bounded control exchange".into());
                }

                maximum.min(self.peers.maxMessageBytes())
            } else { self.peers.maxMessageBytes() };
            let operations = exchangeSpaceControl(self, peer, decoded.operations)?;
            space_projection::requirePayloadFits(&operations, maximum)?;
            Ok(operations)
        })().map_err(spaceCallError);
        encodeTypedReply(&id, result.map(OwnedSequence::new))
    }
    pub async fn acceptSpaceApprovalCall(&self, origin: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
        space_join::receiveApproval(self, origin, request)
    }
    /// Local exit preserves identity and pairing, and bootstraps a new singleton
    /// policy through the same transition used by full Core.
    pub fn leaveDeviceSpace(&self) -> Result<CoreSpace, String> { space_join::leave(self) }
    pub async fn requestDeviceSpaceJoin(&self, peer: String) -> Result<SpaceJoinRequest, String> { space_join::request(self, peer).await }
    pub async fn incomingDeviceSpaceJoins(&self) -> Result<Vec<SpaceJoinRequest>, String> {
        // Before admission this singleton has no remote review sources. Avoid
        // materializing policy trees on every idle UI tick with an empty inbox.
        if self.space.space()?.members.len() == 1 && !space_join::hasInbound(self)? { return Ok(Vec::new()); }
        space_join::incoming(self).await
    }
    pub async fn decideDeviceSpaceJoin(&self, id: String, version: u64, approve: bool) -> Result<SpaceJoinRequest, String> { space_join::decide(self, id, version, approve).await }
    /// Local outgoing approvals, shared by the Core facade and bounded Edge UI.
    pub fn outgoingDeviceSpaceJoins(&self) -> Result<Vec<SpaceJoinRequest>, String> { space_join::outgoing(self) }
    pub async fn refreshDeviceSpaceJoin(&self, id: String) -> Result<SpaceJoinRequest, String> { space_join::refresh(self, id).await }
    pub async fn cancelDeviceSpaceJoin(&self, id: String) -> Result<SpaceJoinRequest, String> { space_join::cancel(self, id).await }
    pub async fn synchronizePreferences(&self) -> Result<(), String> {
        use operit_store::DevicePreferencesStore::{DevicePreferences, DevicePreferencesStore};
        let departures = space_departure::synchronize(self).await;
        let store = DevicePreferencesStore::new(self.storage.clone());
        for peer in self.peers.activePeerNodeIds().map_err(|e| e.to_string())? {
            let Some(scope) = self.spaceChannelScope(&peer).map_err(|e| e.to_string())? else { continue; };
            let local = store.read(&scope, &self.localNodeId)?;
            let response = self.callNode(peer.clone(), CoreCallRequest::new("device-preferences", NODE_SPACE_CONTROL_TARGET,
                "exchangePreferences", toCoreValue(local).map_err(|e| e.to_string())?)).await;
            if response.result.as_ref().is_err_and(|error| error.code == "SPACE_ID_MISMATCH") {
                // A paired endpoint may have exited while its old scoped
                // connection is still alive. Fetch its durable exit receipt.
                space_departure::exchange(self, &peer).await?;
                continue;
            }
            let incoming: DevicePreferences = fromCoreValue(response.result.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            if incoming.spaceId != scope || incoming.nodeId != self.localNodeId
                || self.spaceChannelScope(&peer).map_err(|e| e.to_string())?.as_deref() != Some(scope.as_str()) {
                return Err("Device preference response no longer belongs to this Space".into());
            }
            store.merge(&incoming)?;
        }
        departures
    }
    pub fn spaceChannelScope(&self, peer: &str) -> Result<Option<String>, CoreLinkError> {
        crate::space_policy::spaceChannelScope(&self.space, &self.control, &self.localNodeId, peer)
    }
}
#[async_trait::async_trait(?Send)]
impl NodeSpaceContext for NodeSpaceService {
    fn storage(&self) -> Arc<dyn RuntimeStorageHost> { self.storage.clone() }
    fn spaceStore(&self) -> &CoreSpaceStore { &self.space }
    fn networkControlStore(&self) -> &NetworkControlStore { &self.control }
    fn peers(&self) -> Result<Arc<dyn RuntimePeerService>, String> { Ok(self.peers.clone()) }
    fn localNodeId(&self) -> String { self.localNodeId.clone() }
    fn isSpaceEndpoint(&self) -> bool { true }
    fn nodeIsReachable(&self, node: &str) -> Result<bool, String> {
        Ok(node == self.localNodeId || self.peers.activePeerNodeIds().map_err(|e| e.to_string())?.contains(node))
    }
    async fn callNode(&self, node: String, request: CoreCallRequest) -> CoreCallResponse {
        let local = self.localNodeId.clone();
        let spaceId = if request.target == NODE_SPACE_APPROVAL_TARGET || request.target == NODE_SPACE_CONTROL_TARGET {
            match self.space.space() {
                Ok(space) => space.spaceId,
                Err(error) => return CoreCallResponse::err(request.requestId, CoreLinkError::internal(error)),
            }
        } else { String::new() };
        self.peers.call(&node, RoutedCoreRequest { spaceId, originNodeId: local,
            targetNodeId: node.clone(), ttl: 0, routeKind: RoutedCoreRequestKind::Target, payload: request }).await
    }
    async fn afterJoined(&self, peer: String) -> Result<(), String> {
        // Exchange authority through the shared authenticated session engine
        // after adopting membership. The lightweight endpoint never receives
        // a business replica or advances its replication clock.
        let request = CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId("space-control"),
            NODE_SPACE_CONTROL_TARGET, "exchangeBounded",
            CoreValue::Map(std::collections::BTreeMap::from([
                ("version".into(), toCoreValue(space_projection::VERSION).map_err(|error| error.to_string())?),
                ("maxMessageBytes".into(), toCoreValue(self.peers.maxMessageBytes()).map_err(|error| error.to_string())?),
                ("operations".into(), operationsIntoCoreValue(self.control.currentSpaceOperations()?)?),
            ])),
        );
        let response = self.callNode(peer.clone(), request).await;
        let operations = decodeControlExchange(response.result)?;
        self.control.projectPeerControlOperations(&peer, &operations)?;
        // The joined log carries the Space's whole removal history: stale
        // member projections shrink and the credentials a removal revoked are
        // forgotten in the same step.
        space_join::applyRemovedMemberLocalEffects(self, &operations)?;
        Ok(())
    }
}


/// Exchange only issuer-bound authority commands. Never receives chat, files,
/// profiles or a business replication clock; policy replay remains authoritative.
pub fn acceptSpaceControlCall(service: &dyn NodeSpaceContext, origin: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    if service.isSpaceEndpoint() { space_projection::requirePayloadFits(&request, service.peers()?.maxMessageBytes())?; }
    let space = requireSpaceControlSource(service, origin)?;
    if request.methodName == "exchangePreferences" {
        let incoming: operit_store::DevicePreferencesStore::DevicePreferences = fromCoreValue(request.args).map_err(|e| e.to_string())?;
        if incoming.spaceId != space.spaceId || (incoming.nodeId != origin && incoming.nodeId != service.localNodeId()) {
            return Err("Device preference exchange scope mismatch".into());
        }
        let merged = operit_store::DevicePreferencesStore::DevicePreferencesStore::new(service.storage()).merge(&incoming)?;
        return toCoreValue(merged).map_err(|e| e.to_string());
    }
    if request.methodName == "exchangeBounded" {
        let bounded: space_projection::BoundedControlExchange = fromCoreValue(request.args).map_err(|error| error.to_string())?;
        if bounded.version != space_projection::VERSION || bounded.maxMessageBytes <= space_projection::ENVELOPE_BYTES {
            return Err("SPACE_PROTOCOL_INCOMPATIBLE: Unsupported bounded control exchange".into());
        }
        let maximum = bounded.maxMessageBytes.min(service.peers()?.maxMessageBytes());
        let operations = exchangeSpaceControl(service, origin, bounded.operations)?;
        space_projection::requirePayloadFits(&operations, maximum)?;
        return operationsIntoCoreValue(operations);
    }
    if request.methodName != "exchange" { return Err("Unknown Space control method".into()); }
    let operations: Vec<SyncOperation> = fromCoreValue(request.args).map_err(|error| error.to_string())?;
    operationsIntoCoreValue(exchangeSpaceControl(service, origin, operations)?)
}

fn requireSpaceControlSource(service: &dyn NodeSpaceContext, origin: &str) -> Result<CoreSpace, String> {
    let space = service.spaceStore().space()?;
    let control = service.networkControlStore().currentState()?;
    if !space.members.iter().any(|node| node == origin)
        || !control.memberNodeIds.contains(origin)
        || control.disconnectedNodeIds.contains(origin) {
        return Err("Space control exchange requires an admitted member".into());
    }
    drop(control);
    Ok(space)
}

fn exchangeSpaceControl(service: &dyn NodeSpaceContext, origin: &str, operations: Vec<SyncOperation>) -> Result<Vec<SyncOperation>, String> {
    service.networkControlStore().projectPeerControlOperations(origin, &operations)?;
    // Removals take effect locally whatever path delivered them: this device
    // retires itself when the command targets it, shrinks its own member
    // projection for every other removed device, and forgets the pairing
    // credentials the removal revoked.
    space_join::retireRemovedLocalDevice(service, None)?;
    space_join::applyRemovedMemberLocalEffects(service, &operations)?;
    drop(operations);
    space_projection::requireLegacyControlFits(service)?;
    service.networkControlStore().currentSpaceOperations()
}

/// Decode the required control-plane response. Missing methods and denied calls
/// must remain visible; neither can acknowledge an authority exchange.
pub(crate) fn decodeControlExchange(result: Result<CoreValue, CoreLinkError>) -> Result<Vec<SyncOperation>, String> {
    fromCoreValue(result.map_err(|error| error.to_string())?).map_err(|error| error.to_string())
}

#[cfg(test)]
mod control_exchange_tests {
    use super::*;
    #[test]
    fn encoded_args_decode_routed_headers_without_json_expansion() {
        let args = serde_json::json!({"unknown":{"values":[null,true,-7,"设备"]},"operations":[]});
        let call = serde_json::json!({"requestId":"decode","target":"node.space", "methodName":"observeSpaceSnapshot", "args":args});
        let route = serde_json::json!({"spaceId":"space","originNodeId":"core", "targetNodeId":"edge", "ttl":0, "routeKind":"Target", "payload":call});
        let named = serde_json::json!({"type":"Call","body":{"requestId":"decode", "target":"node.space", "methodName":"observeSpaceSnapshot", "args":route}});
        let compact = serde_json::json!(["Call", ["decode", "node.space", "observeSpaceSnapshot",
            ["space","core","edge",0,"Target",["decode","node.space","observeSpaceSnapshot",args]]]]);
        for value in [named, compact] {
            let mut bytes = operit_link::encodeLink(value).unwrap();
            let decoded = takeEncodedSpaceArgs::<2>(&mut bytes).unwrap();
            assert_eq!(decoded.metadata, args);
            assert!(decoded.operations.is_empty());
            assert_eq!(bytes.capacity(), 0, "release the input frame before policy replay");
        }
        let mut malformed = operit_link::encodeLink(serde_json::json!({"type":"Call","body":{}})).unwrap();
        assert!(takeEncodedSpaceArgs::<2>(&mut malformed).is_err());
    }
    #[test]
    fn encoded_operation_lists_preserve_payloads_and_require_record_metadata() {
        let operation = SyncOperation { opId: "operation".into(), originDeviceId: "issuer".into(), sequence: 2,
            domain: "network_control".into(), entityType: "command".into(), entityId: "identity".into(),
            operation: "set".into(), semantics: operit_store::SyncOperationStore::SyncOperationSemantics::Transaction,
            payload: serde_json::json!({"unknown":{"retained":["设备",-12,1.5]},"u64":u64::MAX}), createdAt: 3, schemaVersion: 1 };
        let expected = vec![operation.clone(), operation];
        let bytes = operit_link::encodeLink(&expected).unwrap();
        assert_eq!(operit_link::decodeLink::<OperationList>(&bytes).unwrap().0, expected);
        let mut malformed = serde_json::to_value(&expected).unwrap();
        malformed[1].as_object_mut().unwrap().remove("sequence");
        let bytes = operit_link::encodeLink(malformed).unwrap();
        assert!(operit_link::decodeLink::<OperationList>(&bytes).is_err());
    }
    #[test]
    fn consuming_snapshot_encoding_matches_generic_wire_values() {
        let payload = serde_json::json!({"unknown": {"text": "设备", "values": [null,true,false,-12,18,1.25]}, "u64":u64::MAX});
        assert_eq!(jsonIntoCoreValue(payload.clone()).unwrap(), toCoreValue(payload).unwrap());
        let operation = SyncOperation {
            opId: "operation".into(), originDeviceId: "issuer".into(), sequence: 2,
            domain: "domain".into(), entityType: "command".into(), entityId: "identity".into(),
            operation: "set".into(), semantics: operit_store::SyncOperationStore::SyncOperationSemantics::Transaction,
            payload: serde_json::json!({"unknown":{"retained":["x",1.5]}}), createdAt: 3, schemaVersion: 1,
        };
        let encoded = serde_json::to_value(vec![operation.clone()]).unwrap();
        let pointer = encoded[0]["payload"]["unknown"]["retained"][0].as_str().unwrap().as_ptr();
        let decoded = operationsFromJson(encoded).unwrap();
        assert_eq!(decoded, vec![operation.clone()]);
        assert_eq!(decoded[0].payload["unknown"]["retained"][0].as_str().unwrap().as_ptr(), pointer);
        for departures in [Vec::new(), vec![operation.clone()]] {
            let snapshot = PeerSpaceSnapshot {
                space: CoreSpace { spaceId: "space".into(), spaceName: "name".into(), spaceRevision: 4, members: vec!["member".into()] },
                deviceProfiles: Vec::new(), topology: Vec::new(), controlOperations: vec![operation.clone()], departures,
            };
            let value = serde_json::to_value(&snapshot).unwrap();
            let pointer = value["controlOperations"][0]["payload"]["unknown"]["retained"][0].as_str().unwrap().as_ptr();
            let restored = snapshotFromJson(value).unwrap();
            assert_eq!(restored.controlOperations[0].payload["unknown"]["retained"][0].as_str().unwrap().as_ptr(), pointer);
            assert_eq!(serde_json::to_value(&restored).unwrap(), serde_json::to_value(&snapshot).unwrap());
            let mut malformed = serde_json::to_value(&snapshot).unwrap();
            malformed["controlOperations"][0].as_object_mut().unwrap().remove("sequence");
            assert!(snapshotFromJson(malformed).is_err());
            let id = operit_link::CoreRequestId::new("snapshot");
            assert_eq!(encodeTypedReply(&id, Ok(OwnedSnapshotReply::from(snapshot.clone()))).unwrap(),
                encodeTypedReply(&id, Ok(snapshot.clone())).unwrap());
            assert_eq!(snapshot.clone().intoCoreValue().unwrap(), toCoreValue(snapshot).unwrap());
        }
        assert_eq!(operationsIntoCoreValue(vec![operation.clone()]).unwrap(), toCoreValue(vec![operation]).unwrap());
    }
    #[test]
    fn reply_records_release_each_payload_before_the_next_and_keep_the_wire_format() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Tracked { index: usize, dropped: Arc<AtomicUsize> }
        impl Serialize for Tracked {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                assert_eq!(self.dropped.load(Ordering::SeqCst), self.index,
                    "previous payload remained alive during the next record");
                self.index.serialize(serializer)
            }
        }
        impl Drop for Tracked {
            fn drop(&mut self) { self.dropped.fetch_add(1, Ordering::SeqCst); }
        }
        let dropped = Arc::new(AtomicUsize::new(0));
        let records = OwnedSequence::new((0..3).map(|index| Tracked { index, dropped: dropped.clone() }).collect());
        let encoded = operit_link::encodeLink(&records).unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 3);
        assert_eq!(encoded, operit_link::encodeLink(vec![0usize, 1, 2]).unwrap());
        assert!(operit_link::encodeLink(&records).is_err(), "consumed records must never silently encode an empty reply");
        let snapshot = PeerSpaceSnapshot {
            space: CoreSpace { spaceId: "space".into(), spaceName: "name".into(), spaceRevision: 2, members: vec!["member".into()] },
            deviceProfiles: Vec::new(), topology: Vec::new(), controlOperations: Vec::new(), departures: Vec::new(),
        };
        assert_eq!(encodeTypedReply(&operit_link::CoreRequestId::new("reply"), Ok(OwnedSnapshotReply::from(snapshot.clone()))).unwrap(),
            encodeTypedReply(&operit_link::CoreRequestId::new("reply"), Ok(snapshot)).unwrap());
    }

    #[test]
    fn expected_space_protocol_failures_keep_a_short_error_envelope() {
        for code in ["SPACE_MESSAGE_LIMIT", "SPACE_PROTOCOL_INCOMPATIBLE", "SPACE_EXPORT_CHANGED",
            "SPACE_EXPORT_CURSOR_INVALID", "SPACE_EXPORT_MIGRATION_REQUIRED", "SPACE_EXPORT_INVALID"] {
            let error = spaceCallError(format!("{code}: retry or reconnect"));
            assert_eq!(error.code, code);
            assert!(error.backtrace.is_none());
            assert!(encodeTypedReply::<CoreValue>(&operit_link::CoreRequestId::new("bounded-error"), Err(error)).unwrap().len() < 512);
        }
        assert_eq!(spaceCallError("Unknown node Space method".into()).code, "INTERNAL_ERROR");
    }
    #[test]
    fn control_exchange_requires_a_valid_acknowledgement() {
        for code in ["METHOD_NOT_FOUND", "SPACE_STORAGE_PERMISSION_DENIED", "PEER_SECURITY", "INTERNAL", "SPACE_ROUTE_NOT_FOUND"] {
            assert!(decodeControlExchange(Err(CoreLinkError::new(code, "failed"))).is_err());
        }
        assert!(decodeControlExchange(Ok(CoreValue::Null)).is_err());
        assert!(decodeControlExchange(Ok(toCoreValue(Vec::<SyncOperation>::new()).unwrap())).unwrap().is_empty());
    }
}
