use operit_util::RuntimeStorageLayout::{RUNTIME_SPACE_CONTROL_PROJECTION_PATH, RUNTIME_SPACE_MEMBERS_DIR_PATH, RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CoreNodeIdentityStore::CoreNodeIdentityStore;
use crate::CoreSpaceStore::{CoreSpaceDeviceProfile, CoreSpaceStore};
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;
use crate::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore};
use crate::NetworkControlReadModel::{lockControlReadModel};
use crate::SyncOperationStore::{
    NewSyncOperation, SyncOperation, SyncOperationOrder, SyncOperationSemantics,
    SyncOperationStore,
};

/// Identifies only preference datasets explicitly retired into endpoint-local authority projections.
fn endpointProjectionPreferencePath(path: &str) -> bool {
    [RUNTIME_SPACE_MEMBERS_DIR_PATH, RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH,
        operit_util::RuntimeStorageLayout::RUNTIME_SPACE_DEVICE_PRESENCE_DIR_PATH,
        operit_util::RuntimeStorageLayout::RUNTIME_SPACE_TOPOLOGY_DIR_PATH]
        .iter().any(|prefix| path.strip_prefix(prefix).is_some_and(|suffix| suffix.starts_with('/'))
            && path.ends_with(".preferences.json"))
}

/// Names the synchronized domain that owns Space authorization and control commands.
pub const NETWORK_CONTROL_SYNC_DOMAIN: &str = "network_control";
const NETWORK_CONTROL_ENTITY_TYPE: &str = "command";
const NETWORK_CONTROL_OPERATION: &str = "apply";

static NETWORK_CONTROL_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Describes one role whose identifier and capabilities are owned by a Space policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlRole {
    pub roleId: String,
    pub displayName: String,
    pub capabilities: BTreeSet<String>,
}

/// Names the capabilities that control identity administration and approval.
pub const NETWORK_IDENTITY_MANAGE_CAPABILITY: &str = "network.identity.manage";
pub const NETWORK_IDENTITY_ASSIGN_CAPABILITY: &str = "network.identity.assign";
pub const NETWORK_APPROVAL_CAPABILITY: &str = "network.approval";
pub const NETWORK_DEVICES_VIEW_CAPABILITY: &str = "network.devices.view";

/// Carries one device identity assignment through the synchronized command log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlIdentityAssignment {
    pub nodeId: String,
    pub roleId: String,
}

/// Carries the mutable policy commands replicated through the Space operation log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkControlCommand {
    Bootstrap {
        initialAdminNodeId: String,
    },
    DefineRole {
        role: NetworkControlRole,
    },
    UpdateRole {
        role: NetworkControlRole,
    },
    RemoveRole {
        roleId: String,
    },
    GrantRole {
        grant: NetworkControlIdentityAssignment,
    },
    RevokeRole {
        nodeId: String,
    },
    AdmitMember {
        nodeId: String,
    },
    AdmitSpace {
        sourceSpaceId: String,
        /// Each admitted device's initial identity in the target Space.
        identities: BTreeMap<String, String>,
    },
    RemoveMember {
        nodeId: String,
        /// The pairing generation the removed device announced while admitted.
        /// Mutual forgetting drops only pairings stamped with this generation:
        /// a pairing the device made after leaving and resetting (its next
        /// generation, for re-admission) survives every replay of this removal.
        /// Older cores omit it and keep the previous forget-only semantics.
        #[serde(default)]
        revokedIncarnation: Option<String>,
    },
    /// An admitted device retires only its own membership, without deleting pairings.
    LeaveSpace,
    DisconnectNode {
        nodeId: String,
    },
    PolicyUpdate {
        policyId: String,
        value: String,
    },
}

/// Records one issuer-bound command before it is encoded as a synchronization operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlCommandRecord {
    pub commandId: String,
    pub spaceId: String,
    pub issuerNodeId: String,
    #[serde(deserialize_with = "deserializeControlCommand")]
    pub command: NetworkControlCommand,
}

/// Normalize the published nodeIds format at the read boundary; stored operations stay immutable.
#[allow(non_snake_case)]
fn deserializeControlCommand<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<NetworkControlCommand, D::Error> {
    use serde::de::Error;
    let mut value = serde_json::Value::deserialize(deserializer)?;
    if let Some(admission) = value.get_mut("admit_space").and_then(serde_json::Value::as_object_mut) {
        if let Some(nodes) = admission.remove("nodeIds") {
            if admission.contains_key("identities") {
                return Err(D::Error::custom("Space admission cannot contain both nodeIds and identities"));
            }
            let nodes: BTreeSet<String> = serde_json::from_value(nodes).map_err(D::Error::custom)?;
            let identities: BTreeMap<_, _> = nodes.into_iter().map(|node| (node, "user")).collect();
            admission.insert("identities".into(), serde_json::to_value(identities).map_err(D::Error::custom)?);
        }
    }
    serde_json::from_value(value).map_err(D::Error::custom)
}

/// Only decoded historical admissions retain the original join-only authorization.
#[allow(non_snake_case)]
fn isLegacyAdmission(operation: &SyncOperation) -> bool {
    operation.payload["command"]["admit_space"].get("nodeIds").is_some()
}

/// Reports the local authorization decision for one replicated control command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlAuditRecord {
    pub commandId: String,
    pub spaceId: String,
    pub issuerNodeId: String,
    pub originNodeId: String,
    pub accepted: bool,
    pub reason: String,
    pub summary: String,
    pub recordedAt: i64,
}

/// Materializes the authorization state selected by accepted control commands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlState {
    pub spaceId: String,
    pub initialized: bool,
    pub memberNodeIds: BTreeSet<String>,
    pub roles: BTreeMap<String, NetworkControlRole>,
    pub deviceIdentityIds: BTreeMap<String, String>,
    pub disconnectedNodeIds: BTreeSet<String>,
    pub policies: BTreeMap<String, String>,
}

/// Owns the compact, auditable authorization policy for one synchronized Space.
#[derive(Clone)]
pub struct NetworkControlStore {
    spaceStore: CoreSpaceStore,
    syncOperationStore: SyncOperationStore,
    controlProjection: CoreNodeStateStore,
    nodeLocalOnly: bool,
    localNodeId: String,
}

impl NetworkControlStore {
    /// Opens the Space control store over one runtime storage host.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Result<Self, String> {
        let localNodeId = CoreNodeIdentityStore::new(storage.clone())
            .initialize()?
            .nodeId;
        Ok(Self::withLocalNodeId(storage, localNodeId))
    }

    /// Constructs the handle without reading or replaying Space data.
    #[allow(non_snake_case)]
    pub fn withLocalNodeId(storage: Arc<dyn RuntimeStorageHost>, localNodeId: String) -> Self {
        let syncOperationStore = SyncOperationStore::new(storage.clone(), RUNTIME_SYNC_DIR_PATH);
        Self {
            spaceStore: CoreSpaceStore::new(storage.clone()),
            syncOperationStore,
            controlProjection: Self::projectionStore(storage),
            nodeLocalOnly: false,
            localNodeId,
        }
    }

    /// Opens the versioned local projection of synchronized control operations.
    pub fn projectionStore(storage: Arc<dyn RuntimeStorageHost>) -> CoreNodeStateStore {
        CoreNodeStateStore::newWithStorage(storage, RUNTIME_SPACE_CONTROL_PROJECTION_PATH)
            .withSchema(2, |version, preferences| match version {
                0 => Ok(()),
                1 => {
                    for (key, value) in preferences.entries() {
                        if key == crate::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME { continue; }
                        let encoded = serde_json::from_value::<String>(value)?;
                        let operation: SyncOperation = serde_json::from_str(&encoded)?;
                        preferences.set(&stringPreferencesKey(&key), serde_json::to_value(operation)?);
                    }
                    Ok(())
                }
                from => Err(crate::PreferencesDataStore::PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            })
    }

    /// Drops the endpoint's old projection by path, even if it cannot decode.
    /// Full Core journals are scoped by Space id and retain their business history.
    #[allow(non_snake_case)]
    pub fn resetLocalProjection(&self) -> Result<(), String> {
        let _lock = networkControlMutationLock().lock().map_err(|error| error.to_string())?;
        self.controlProjection.delete().map_err(|error| error.to_string())?;
        self.syncOperationStore.replaceProjectedControlOperations(&self.controlProjection, self.nodeLocalOnly)
    }

    /// Endpoint authority is durable node-local control state, not a business
    /// replication journal. No sync clocks/device indices are created here.
    #[allow(non_snake_case)]
    pub fn newNodeLocal(storage: Arc<dyn RuntimeStorageHost>) -> Result<Self, String> {
        let mut store = Self::new(storage.clone())?;
        store.spaceStore = CoreSpaceStore::newNodeLocal(storage);
        store.nodeLocalOnly = true;
        Ok(store)
    }

    /// Opens the Space control store over the process-wide runtime storage host.
    pub fn native() -> Result<Self, String> {
        Self::new(defaultRuntimeStorageHost())
    }

    /// Creates the initial administrator command for a newly created single-device Space.
    #[allow(non_snake_case)]
    pub fn bootstrapCurrentSpace(&self) -> Result<NetworkControlState, String> {
        let _lock = networkControlMutationLock()
            .lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        let space = self.spaceStore.initialize()?;
        if space.members.len() != 1 || space.members.first() != Some(&self.localNodeId) {
            return Err(
                "only a new single-device Space can bootstrap its control policy".to_string(),
            );
        }
        let state = self.stateForSpace(&space.spaceId)?;
        if state.initialized {
            return Err("Space control policy is already initialized".to_string());
        }
        self.appendLocalCommand(
            &space.spaceId,
            NetworkControlCommand::Bootstrap {
                initialAdminNodeId: self.localNodeId.clone(),
            },
        )?;
        self.stateForSpace(&space.spaceId)
    }

    /// Returns the initialized Space control policy, bootstrapping only the creator's new Space.
    #[allow(non_snake_case)]
    pub fn initializeCurrentSpace(&self) -> Result<NetworkControlState, String> {
        let state = self.currentState()?;
        if state.initialized {
            return Ok(state);
        }
        let space = self.spaceStore.initialize()?;
        if space.members.len() == 1 && space.members.first() == Some(&self.localNodeId) {
            return self.bootstrapCurrentSpace();
        }
        Ok(state)
    }

    /// Returns the accepted authorization state for the current Space.
    #[allow(non_snake_case)]
    pub fn currentState(&self) -> Result<NetworkControlState, String> {
        let space = self.spaceStore.initialize()?;
        self.stateForSpace(&space.spaceId)
    }

    /// Returns the complete accepted and rejected command audit for the current Space.
    pub fn audit(&self) -> Result<Vec<NetworkControlAuditRecord>, String> {
        let space = self.spaceStore.initialize()?;
        self.materialize(&space.spaceId).map(|(_, audit)| audit)
    }

    /// Returns whether one node has one capability for an optional managed target.
    #[allow(non_snake_case)]
    pub fn nodeHasCapability(
        &self,
        nodeId: &str,
        capability: &str,
        targetNodeId: Option<&str>,
    ) -> Result<bool, String> {
        validateNodeId(nodeId)?;
        validateCapability(capability)?;
        if let Some(targetNodeId) = targetNodeId {
            validateNodeId(targetNodeId)?;
        }
        let state = self.currentState()?;
        Ok(hasCapability(&state, nodeId, capability, targetNodeId))
    }

    /// Returns whether a node is prohibited from being used as a direct connection or route hop.
    #[allow(non_snake_case)]
    pub fn nodeIsDisconnected(&self, nodeId: &str) -> Result<bool, String> {
        validateNodeId(nodeId)?;
        Ok(self
            .currentState()?
            .disconnectedNodeIds
            .contains(nodeId))
    }

    /// Returns the current Space members explicitly permitted to carry a routed later hop.
    #[allow(non_snake_case)]
    pub fn relayNodeIds(&self) -> Result<BTreeSet<String>, String> {
        let space = self.spaceStore.initialize()?;
        let state = self.stateForSpace(&space.spaceId)?;
        Ok(space
            .members
            .into_iter()
            .filter(|nodeId| hasCapability(&state, nodeId, "network.relay", None))
            .collect())
    }

    /// Returns the routing policy inputs derived from one consistent policy replay.
    #[allow(non_snake_case)]
    pub fn routingPolicySnapshot(
        &self,
    ) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
        let space = self.spaceStore.initialize()?;
        let state = self.stateForSpace(&space.spaceId)?;
        let blockedNodeIds = state.disconnectedNodeIds.clone();
        let relayNodeIds = space
            .members
            .into_iter()
            .filter(|nodeId| hasCapability(&state, nodeId, "network.relay", None))
            .collect();
        Ok((blockedNodeIds, relayNodeIds))
    }

    /// Defines one custom role after checking that the issuer can define every requested ability.
    #[allow(non_snake_case)]
    pub fn defineRole(&self, role: NetworkControlRole) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::DefineRole { role })
    }

    /// Updates one existing custom role after checking that the issuer can define every requested ability.
    #[allow(non_snake_case)]
    pub fn updateRole(&self, role: NetworkControlRole) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::UpdateRole { role })
    }

    /// Removes one custom role. Devices still holding the role fall back to
    /// the default user identity, mirroring the RevokeRole guard that keeps
    /// every member holding an identity at all times.
    #[allow(non_snake_case)]
    pub fn removeRole(&self, roleId: String) -> Result<SyncOperation, String> {
        validateIdentifier("role id", &roleId, 120)?;
        self.submitLocalCommand(NetworkControlCommand::RemoveRole { roleId })
    }

    /// Sets one existing identity on a device after checking capability limits.
    #[allow(non_snake_case)]
    pub fn setIdentity(
        &self,
        assignment: NetworkControlIdentityAssignment,
    ) -> Result<SyncOperation, String> {
        self.requireKnownSpaceMember(&assignment.nodeId)?;
        self.submitLocalCommand(NetworkControlCommand::GrantRole { grant: assignment })
    }

    /// Resets one device's identity to the default user identity. Members
    /// keep an identity at all times: an identity-less member would lose
    /// every capability - including the ones its own UI depends on - with
    /// no in-band recovery path.
    #[allow(non_snake_case)]
    pub fn clearIdentity(&self, nodeId: String) -> Result<SyncOperation, String> {
        self.requireKnownSpaceMember(&nodeId)?;
        self.submitLocalCommand(NetworkControlCommand::RevokeRole { nodeId })
    }

    /// Forgets a member completely: every policy record for the device is
    /// cleared, so it may only return through a fresh pairing and a new join
    /// approval. Replay is idempotent; capability and sole-administrator
    /// guards stay in the audited command replay.
    #[allow(non_snake_case)]
    pub fn removeMember(&self, nodeId: String) -> Result<SyncOperation, String> {
        validateNodeId(&nodeId)?;
        // The removal revokes the generation this device announced while it
        // was admitted; a stale local profile simply carries an older value.
        let revokedIncarnation = self.spaceStore.deviceProfiles()?
            .get(&nodeId).and_then(|profile| profile.pairingIncarnation.clone());
        self.submitLocalCommand(NetworkControlCommand::RemoveMember { nodeId, revokedIncarnation })
    }

    /// Records a local exit before the node switches Spaces. Retrying an
    /// interrupted exit reuses its receipt; a later admission permits a new exit.
    #[allow(non_snake_case)]
    pub fn leaveCurrentSpace(&self) -> Result<Option<SyncOperation>, String> {
        let _lock = networkControlMutationLock().lock().map_err(|error| error.to_string())?;
        let space = self.spaceStore.initialize()?;
        let state = self.stateForSpace(&space.spaceId)?;
        if !state.memberNodeIds.contains(&self.localNodeId) {
            return self.latestLocalDeparture(&space.spaceId);
        }
        self.appendLocalCommand(&space.spaceId, NetworkControlCommand::LeaveSpace).map(Some)
    }

    /// An exit still awaiting its local Space switch, rather than an older
    /// exit superseded by a subsequent admission or administrative removal.
    #[allow(non_snake_case)]
    pub fn committedLocalDeparture(&self) -> Result<Option<SyncOperation>, String> {
        let space = self.spaceStore.initialize()?;
        if self.stateForSpace(&space.spaceId)?.memberNodeIds.contains(&self.localNodeId) { return Ok(None); }
        self.latestLocalDeparture(&space.spaceId)
    }

    #[allow(non_snake_case)]
    fn latestLocalDeparture(&self, spaceId: &str) -> Result<Option<SyncOperation>, String> {
        for operation in self.orderedCommands(spaceId)?.into_iter().rev() {
            let record = decodeControlOperation(&operation)?;
            match record.command {
                NetworkControlCommand::LeaveSpace if record.issuerNodeId == self.localNodeId
                    && operation.originDeviceId == self.localNodeId => return Ok(Some(operation)),
                NetworkControlCommand::RemoveMember { nodeId, .. } | NetworkControlCommand::AdmitMember { nodeId }
                    if nodeId == self.localNodeId => break,
                NetworkControlCommand::AdmitSpace { identities, .. } if identities.contains_key(&self.localNodeId) => break,
                _ => {},
            }
        }
        Ok(None)
    }

    /// Admits one authenticated device as a member of the current Space.
    #[allow(non_snake_case)]
    pub fn admitMember(&self, nodeId: String) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::AdmitMember { nodeId })
    }

    /// Approves every member of one source Space as one auditable policy command.
    pub fn admitSpace(&self, sourceSpaceId: String, identities: BTreeMap<String, String>) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::AdmitSpace { sourceSpaceId, identities })
    }

    /// Validates and replays a received policy without publishing any received operation.
    pub fn validateSpacePolicy(&self, spaceId: &str, operations: &[SyncOperation]) -> Result<NetworkControlState, String> {
        validateSpaceId(spaceId)?;
        let mut ordered = operations.to_vec();
        for operation in &ordered { validateControlOperation(operation)?; }
        ordered.sort_by_key(controlOperationOrder);
        self.replayCommands(spaceId, &ordered, false).map(|(state, _)| state)
    }

    /// Proves that an authorized target command admitted the complete source membership.
    pub fn validateSpaceAdmission(&self, targetSpaceId: &str, sourceSpaceId: &str,
        members: &BTreeSet<String>, operations: &[SyncOperation]) -> Result<(), String> {
        let state = self.validateSpacePolicy(targetSpaceId, operations)?;
        if !members.is_subset(&state.memberNodeIds) {
            return Err("Merged Space membership is not authorized by the target policy".into());
        }
        let mut ordered = operations.to_vec();
        ordered.sort_by_key(controlOperationOrder);
        for (index, operation) in ordered.iter().enumerate() {
            let record = decodeControlOperation(operation)?;
            if record.spaceId != targetSpaceId { continue; }
            if let NetworkControlCommand::AdmitSpace { sourceSpaceId: source, identities } = &record.command {
                if source == sourceSpaceId && members.iter().all(|node| identities.contains_key(node)) {
                    let (mut before, _) = self.replayCommands(targetSpaceId, &ordered[..index], false)?;
                    if authorizeAndApplyCommand(&mut before, &record, &operation.originDeviceId, isLegacyAdmission(operation)).is_ok() {
                        return Ok(());
                    }
                }
            }
        }
        Err("Target Space has no approved admission for this source Space".into())
    }

    /// Reuses this explicit review's durable admission after a partial write.
    /// The key is bound to the claimed request/version, not just the member:
    /// removing and later readmitting a member must still create a new command.
    #[allow(non_snake_case)]
    pub fn admitSpaceForReview(&self, sourceSpaceId: String, identities: BTreeMap<String, String>, reviewKey: &str) -> Result<SyncOperation, String> {
        let _lock = networkControlMutationLock()
            .lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        let space = self.spaceStore.initialize()?;
        let state = self.stateForSpace(&space.spaceId)?;
        let command = NetworkControlCommand::AdmitSpace { sourceSpaceId, identities };
        authorizeCommand(&state, &self.localNodeId, &command)?;
        let commandId = format!("control-review-{reviewKey}");
        for operation in self.orderedCommands(&space.spaceId)? {
            if operation.entityId != commandId || operation.originDeviceId != self.localNodeId { continue; }
            let record: NetworkControlCommandRecord = serde_json::from_value(operation.payload.clone()).map_err(|error| error.to_string())?;
            if record.spaceId != space.spaceId || record.issuerNodeId != self.localNodeId || record.command != command {
                return Err("Stored admission differs from the claimed review".into());
            }
            // The log append may have succeeded before clock/version metadata.
            if !self.nodeLocalOnly {
                self.syncOperationStore.recoverAppendedOperation(&operation).map_err(|error| error.to_string())?;
            }
            return Ok(operation);
        }
        self.appendLocalCommandWithId(&space.spaceId, command, commandId)
    }

    /// Returns the current Space control commands in authorization order.
    #[allow(non_snake_case)]
    pub fn currentSpaceOperations(&self) -> Result<Vec<SyncOperation>, String> {
        let space = self.spaceStore.initialize()?;
        self.orderedCommands(&space.spaceId)
    }

    /// Counts the peer control operations this store currently projects.
    ///
    /// Projected operations never advance the replication journal, so this
    /// count is the cheap local signal that this device learned authority
    /// changes through relaying and owes its peers a projection exchange.
    #[allow(non_snake_case)]
    pub fn projectionEntryCount(&self) -> Result<usize, String> {
        Ok(self
            .controlProjection
            .data()
            .map_err(|error| error.to_string())?
            .iterEntries()
            .filter(|(key, _)| *key != crate::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME)
            .count())
    }

    /// Reads an archived local policy without transmitting unrelated Spaces.
    #[allow(non_snake_case)]
    pub fn spaceOperations(&self, spaceId: &str) -> Result<Vec<SyncOperation>, String> {
        self.orderedCommands(spaceId)
    }

    /// Prevents a device from being used as a direct connection or transit route.
    #[allow(non_snake_case)]
    pub fn disconnectNode(&self, nodeId: String) -> Result<SyncOperation, String> {
        self.requireKnownSpaceMember(&nodeId)?;
        self.submitLocalCommand(NetworkControlCommand::DisconnectNode { nodeId })
    }

    /// Updates one policy value after the issuer proves policy-management capability.
    #[allow(non_snake_case)]
    pub fn updatePolicy(&self, policyId: String, value: String) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::PolicyUpdate { policyId, value })
    }

    /// Applies one received control command without emitting a second local command.
    #[allow(non_snake_case)]
    pub fn applySyncedOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        let _lock = networkControlMutationLock()
            .lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        validateControlOperation(operation)?;
        if self.nodeLocalOnly { return self.projectControlOperationsUnlocked(std::slice::from_ref(operation)); }
        self.syncOperationStore
            .appendUnobservedOperation(operation)
            .map_err(|error| error.to_string())?;
        // Acknowledge the control sequence on its own clock so the same page is
        // not exported on every synchronization pass. Business sequences stay
        // behind this command so earlier chat or file operations are not lost.
        self.syncOperationStore
            .observeNetworkControlOperation(operation)
            .map_err(|error| error.to_string())
    }

    /// Applies a bootstrap control command without local conflict filtering.
    #[allow(non_snake_case)]
    pub fn applyBootstrapOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        if !self.nodeLocalOnly { return self.applySyncedOperation(operation); }
        let _lock = networkControlMutationLock().lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        // An explicit bootstrap may install the validated destination policy
        // before its membership is published. Ordinary peer projection remains
        // restricted to the current Space and authenticated issuer below.
        let spaceId = decodeControlOperation(operation)?.spaceId;
        self.projectControlOperationsForSpaceUnlocked(std::slice::from_ref(operation), &spaceId)
    }

    /// Import authority facts without advancing a business replication clock.
    /// In particular, receiving a later role grant must not hide earlier chat
    /// or file operations if this device subsequently becomes a storage node.
    #[allow(non_snake_case)]
    pub fn projectControlOperations(&self, operations: &[SyncOperation]) -> Result<(), String> {
        let _lock = networkControlMutationLock().lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        self.projectControlOperationsUnlocked(operations)
    }

    /// An authenticated non-administrator may repeat already known authority
    /// facts, but must not invent commands attributed to another device.
    #[allow(non_snake_case)]
    pub fn projectPeerControlOperations(&self, peer: &str, operations: &[SyncOperation]) -> Result<(), String> {
        let _lock = networkControlMutationLock().lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        let spaceId = self.spaceStore.space()?.spaceId;
        let known = self.orderedCommands(&spaceId)?;
        let state = self.stateForSpace(&spaceId)?;
        if !state.memberNodeIds.contains(peer)
            || state.disconnectedNodeIds.contains(peer) { return Err("Control source is not an admitted member".into()); }
        let canRelayAuthority = hasCapability(&state, peer, "*", None);
        for operation in operations {
            if known.iter().any(|old| old == operation) { continue; }
            let record = decodeControlOperation(operation)?;
            if matches!(record.command, NetworkControlCommand::Bootstrap { .. }) {
                return Err("An admitted member cannot replace the Space authority root".into());
            }
            if !canRelayAuthority {
                if operation.originDeviceId != peer || record.issuerNodeId != peer {
                    return Err("Control source cannot introduce another issuer's authority command".into());
                }
                // Replay order must not let a formerly privileged member use a
                // backdated command to resurrect authority revoked in the present.
                authorizeAndApplyCommand(&mut state.clone(), &record, &operation.originDeviceId, isLegacyAdmission(operation))?;
            }
        }
        self.projectControlOperationsUnlocked(operations)
    }

    /// One-time retirement of the old endpoint's replica machinery. Validate
    /// every journal first, durably preserve all authority, then remove exact
    /// journal/metadata keys only. Never touch credentials or business records.
    #[allow(non_snake_case)]
    pub fn migrateNodeLocalProjection(&self, storage: &dyn RuntimeStorageHost) -> Result<(), String> {
        if !self.nodeLocalOnly { return Err("Replica retirement is endpoint-only".into()); }
        let _lock = networkControlMutationLock().lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        let entries = storage.list("runtime/sync/").map_err(|error| error.to_string())?;
        if entries.is_empty() { return Ok(()); }
        let mut preferences = self.controlProjection.data().map_err(|error| error.to_string())?;
        for entry in &entries {
            if entry.isDirectory { continue; }
            if entry.path.starts_with("runtime/sync/operations/") && entry.path.ends_with(".jsonl") {
                let bytes = storage.readBytes(&entry.path).map_err(|error| error.to_string())?;
                let raw = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
                for line in raw.lines().filter(|line| !line.trim().is_empty()) {
                    let operation: SyncOperation = serde_json::from_str(line).map_err(|error| error.to_string())?;
                    if operation.domain == NETWORK_CONTROL_SYNC_DOMAIN {
                        validateControlOperation(&operation)?;
                        let key = stringPreferencesKey(&operation.opId);
                        if let Some(old) = preferences.get(&key) {
                            let old: SyncOperation = serde_json::from_value(old.clone()).map_err(|error| error.to_string())?;
                            if old != operation { return Err("Conflicting legacy authority command".into()); }
                        } else { preferences.set(&key, serde_json::to_value(&operation).map_err(|error| error.to_string())?); }
                    } else {
                        let record = crate::PreferencesDataStore::PreferencesSyncedEntry::fromOperation(&operation)
                            .map_err(|_| "Endpoint journal contains business operations; refusing automatic retirement")?;
                        let path = record.storagePath();
                        if !endpointProjectionPreferencePath(path) {
                            return Err("Endpoint journal contains business operations; refusing automatic retirement".into());
                        }
                    }
                }
            } else if entry.path == "runtime/sync/preference_outboxes.json" {
                let registry: BTreeMap<String, bool> = serde_json::from_slice(&storage.readBytes(&entry.path).map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
                for (path, encrypted) in registry {
                    if encrypted || !endpointProjectionPreferencePath(&path) {
                        return Err("Endpoint outbox contains business operations; refusing automatic retirement".into());
                    }
                }
            } else if !["runtime/sync/clocks.json", "runtime/sync/devices.json", "runtime/sync/entity_versions.jsonl",
                "runtime/sync/local_device_id", "runtime/sync/export_floors.json"].contains(&entry.path.as_str()) {
                return Err("Unknown endpoint sync record; refusing automatic retirement".into());
            }
        }
        // The authority copy must be durable before the first deletion; a failed
        // migration can be retried without losing membership or reviewer rights.
        self.controlProjection.replaceRecoverably(preferences).map_err(|error| error.to_string())?;
        self.syncOperationStore.replaceProjectedControlOperations(&self.controlProjection, false)?;
        for entry in entries.into_iter().filter(|entry| !entry.isDirectory) {
            storage.delete(&entry.path, false).map_err(|error| error.to_string())?;
        }
        self.syncOperationStore.replaceProjectedControlOperations(&self.controlProjection, true)?;
        Ok(())
    }

    /// Endpoint projections are caches for the current Space, not archives of
    /// retired Spaces. Keep every current authority command; full Core's audit
    /// and synchronization journal remain untouched.
    pub fn pruneNodeLocalProjection(&self) -> Result<(), String> {
        if !self.nodeLocalOnly { return Ok(()); }
        let _lock = networkControlMutationLock().lock().map_err(|e| e.to_string())?;
        let spaceId = self.spaceStore.space()?.spaceId;
        let mut preferences = self.controlProjection.data().map_err(|e| e.to_string())?;
        let mut retired = Vec::new();
        for (id, encoded) in preferences.iterEntries() {
            if id == crate::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME { continue; }
            let operation: SyncOperation = serde_json::from_value(encoded.clone()).map_err(|e| e.to_string())?;
            if decodeControlOperation(&operation)?.spaceId != spaceId { retired.push(id.to_owned()); }
        }
        if !retired.is_empty() {
            for id in retired { preferences.remove(&stringPreferencesKey(&id)); }
            self.controlProjection.replaceRecoverably(preferences).map_err(|e| e.to_string())?;
            self.syncOperationStore.replaceProjectedControlOperations(&self.controlProjection, false)?;
        }
        Ok(())
    }

    fn projectControlOperationsUnlocked(&self, operations: &[SyncOperation]) -> Result<(), String> {
        if operations.is_empty() { return Ok(()); }
        let spaceId = self.spaceStore.space()?.spaceId;
        self.projectControlOperationsForSpaceUnlocked(operations, &spaceId)
    }

    fn projectControlOperationsForSpaceUnlocked(&self, operations: &[SyncOperation], spaceId: &str) -> Result<(), String> {
        let mut preferences = self.controlProjection.data().map_err(|error| error.to_string())?;
        let existing = preferences.clone();
        for operation in operations {
            validateControlOperation(operation)?;
            let record = decodeControlOperation(operation)?;
            if record.spaceId != spaceId { return Err("Control projection belongs to another Space".into()); }
            let key = stringPreferencesKey(&operation.opId);
            let encoded = serde_json::to_value(operation).map_err(|error| error.to_string())?;
            if let Some(old) = preferences.get(&key) {
                let old: SyncOperation = serde_json::from_value(old.clone()).map_err(|error| error.to_string())?;
                if old != *operation { return Err("Conflicting immutable Space control operation".into()); }
            } else { preferences.set(&key, encoded); }
        }
        if existing.entries() != preferences.entries() {
            self.controlProjection.replaceRecoverably(preferences).map_err(|error| error.to_string())?;
            self.syncOperationStore.replaceProjectedControlOperations(&self.controlProjection, false)?;
            crate::SyncOperationStore::publishSyncMutation();
        }
        Ok(())
    }

    /// Submits a local command after materializing the latest Space policy under one lock.
    #[allow(non_snake_case)]
    fn submitLocalCommand(&self, command: NetworkControlCommand) -> Result<SyncOperation, String> {
        let _lock = networkControlMutationLock()
            .lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        let space = self.spaceStore.initialize()?;
        let state = self.stateForSpace(&space.spaceId)?;
        authorizeCommand(&state, &self.localNodeId, &command)?;
        self.appendLocalCommand(&space.spaceId, command)
    }

    /// Rejects a policy mutation whose target device does not belong to this Space.
    #[allow(non_snake_case)]
    fn requireKnownSpaceMember(&self, nodeId: &str) -> Result<(), String> {
        validateNodeId(nodeId)?;
        if self.currentState()?.memberNodeIds.contains(nodeId) {
            return Ok(());
        }
        Err(format!(
            "device has not been admitted to the current Space: {nodeId}"
        ))
    }

    /// Appends one issuer-bound command into the common synchronization operation log.
    #[allow(non_snake_case)]
    fn appendLocalCommand(
        &self,
        spaceId: &str,
        command: NetworkControlCommand,
    ) -> Result<SyncOperation, String> {
        self.appendLocalCommandWithId(spaceId, command, format!("control-{}", Uuid::new_v4().simple()))
    }

    #[allow(non_snake_case)]
    fn appendLocalCommandWithId(&self, spaceId: &str, command: NetworkControlCommand, commandId: String) -> Result<SyncOperation, String> {
        let record = NetworkControlCommandRecord {
            commandId,
            spaceId: spaceId.to_string(),
            issuerNodeId: self.localNodeId.clone(),
            command,
        };
        // Field validation must precede the durable append. decode-time validation
        // alone lets a malformed command into the log, where every later replay
        // fails and permanently breaks this Space's control-plane materialization.
        validateCommandRecord(&record)?;
        if self.nodeLocalOnly {
            let sequence = self.orderedCommands(spaceId)?.iter()
                .filter(|op| op.originDeviceId == self.localNodeId)
                .map(|op| op.sequence).max().unwrap_or(0).checked_add(1)
                .ok_or("Space control sequence exhausted")?;
            let operation = SyncOperation {
                opId: record.commandId.clone(), originDeviceId: self.localNodeId.clone(), sequence,
                domain: NETWORK_CONTROL_SYNC_DOMAIN.into(), entityType: NETWORK_CONTROL_ENTITY_TYPE.into(),
                entityId: record.commandId.clone(), operation: NETWORK_CONTROL_OPERATION.into(),
                semantics: SyncOperationSemantics::Transaction,
                payload: serde_json::to_value(record).map_err(|error| error.to_string())?,
                createdAt: operit_host_api::TimeUtils::currentTimeMillis(), schemaVersion: 1,
            };
            self.projectControlOperationsUnlocked(std::slice::from_ref(&operation))?;
            return Ok(operation);
        }
        self.syncOperationStore
            .appendLocalOperation(
                &self.localNodeId,
                NewSyncOperation {
                    domain: NETWORK_CONTROL_SYNC_DOMAIN.to_string(),
                    entityType: NETWORK_CONTROL_ENTITY_TYPE.to_string(),
                    entityId: record.commandId.clone(),
                    operation: NETWORK_CONTROL_OPERATION.to_string(),
                    semantics: SyncOperationSemantics::Transaction,
                    payload: serde_json::to_value(record).map_err(|error| error.to_string())?,
                },
            )
            .map_err(|error| error.to_string())
    }

    /// Reads the state maintained by committed control mutations.
    #[allow(non_snake_case)]
    fn stateForSpace(&self, spaceId: &str) -> Result<NetworkControlState, String> {
        lockControlReadModel(&self.syncOperationStore.networkControlReadModel().map_err(|error| error.to_string())?).map_err(|error| error.to_string())?.state(spaceId)
    }

    /// Replays the current Space command log in deterministic order and emits its audit decisions.
    fn materialize(
        &self,
        spaceId: &str,
    ) -> Result<(NetworkControlState, Vec<NetworkControlAuditRecord>), String> {
        let commands = self.orderedCommands(spaceId)?;
        self.replayCommands(spaceId, &commands, true)
    }

    /// Reads the exact policy revision in deterministic authorization order.
    fn orderedCommands(&self, spaceId: &str) -> Result<Vec<SyncOperation>, String> {
        validateSpaceId(spaceId)?;
        lockControlReadModel(&self.syncOperationStore.networkControlReadModel().map_err(|error| error.to_string())?).map_err(|error| error.to_string())?.commands(spaceId)
    }

    /// Replays authorization commands and reads display profiles only for explicit audits.
    fn replayCommands(
        &self,
        spaceId: &str,
        commands: &[SyncOperation],
        collectAudit: bool,
    ) -> Result<(NetworkControlState, Vec<NetworkControlAuditRecord>), String> {
        let profiles = if collectAudit { Some(self.spaceStore.deviceProfiles()?) } else { None };
        replayControlCommands(spaceId, commands, profiles.as_ref())
    }
}

pub(crate) fn emptyControlState(spaceId: &str) -> NetworkControlState {
    NetworkControlState {
        spaceId: spaceId.to_string(),
        initialized: false,
        memberNodeIds: BTreeSet::new(),
        roles: builtinRoles(),
        deviceIdentityIds: BTreeMap::new(),
        disconnectedNodeIds: BTreeSet::new(),
        policies: BTreeMap::new(),
    }
}

/// Replays only control commands; regular state queries never call this.
pub(crate) fn replayControlCommands(
    spaceId: &str,
    commands: &[SyncOperation],
    profiles: Option<&BTreeMap<String, CoreSpaceDeviceProfile>>,
) -> Result<(NetworkControlState, Vec<NetworkControlAuditRecord>), String> {
    let mut state = emptyControlState(spaceId);
    let mut audit = Vec::new();
    for operation in commands {
        let record = decodeControlOperation(&operation)?;
        if record.spaceId != spaceId {
            continue;
        }
        let summary = profiles
            .map(|profiles| controlAuditSummary(&record.command, &state, profiles))
            .transpose()?;
        let result = authorizeAndApplyCommand(
            &mut state,
            &record,
            &operation.originDeviceId,
            isLegacyAdmission(operation),
        );
        let (accepted, reason) = match result {
            Ok(()) => (true, "accepted".to_string()),
            Err(reason) => (false, reason),
        };
        if let Some(summary) = summary {
            audit.push(NetworkControlAuditRecord {
                commandId: record.commandId,
                spaceId: record.spaceId,
                issuerNodeId: record.issuerNodeId,
                originNodeId: operation.originDeviceId.clone(),
                accepted,
                reason,
                summary,
                recordedAt: operation.createdAt,
            });
        }
    }
    Ok((state, audit))
}

/// Returns the process-wide mutex that serializes local policy decisions and writes.
#[allow(non_snake_case)]
fn networkControlMutationLock() -> &'static Mutex<()> {
    NETWORK_CONTROL_MUTATION_LOCK.get_or_init(|| Mutex::new(()))
}

/// Returns the immutable built-in roles exposed alongside user-defined roles.
#[allow(non_snake_case)]
fn builtinRoles() -> BTreeMap<String, NetworkControlRole> {
    [
        builtinRole("admin", "Administrator", ["*"]),
        builtinRole(
            "user",
            "User",
            [
                NETWORK_DEVICES_VIEW_CAPABILITY,
                "network.relay",
                "runtime.execute",
                "storage.provide",
            ],
        ),
        builtinRole("relay", "Relay", [NETWORK_DEVICES_VIEW_CAPABILITY, "network.relay", "storage.provide"]),
        builtinRole("edge", "Edge", [NETWORK_DEVICES_VIEW_CAPABILITY, "runtime.execute"]),
    ]
    .into_iter()
    .map(|role| (role.roleId.clone(), role))
    .collect()
}

/// Creates one fixed built-in role definition.
#[allow(non_snake_case)]
fn builtinRole(
    roleId: &str,
    displayName: &str,
    capabilities: impl IntoIterator<Item = &'static str>,
) -> NetworkControlRole {
    NetworkControlRole {
        roleId: roleId.to_string(),
        displayName: displayName.to_string(),
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
    }
}

/// Builds a concise human-readable audit description for the management UI.
fn controlAuditSummary(
    command: &NetworkControlCommand,
    state: &NetworkControlState,
    profiles: &std::collections::BTreeMap<String, CoreSpaceDeviceProfile>,
) -> Result<String, String> {
    let summary = match command {
        NetworkControlCommand::Bootstrap { initialAdminNodeId } => format!(
            "Bootstrap control; administrator {}",
            controlDeviceLabel(profiles, initialAdminNodeId)?
        ),
        NetworkControlCommand::DefineRole { role } => format!("Define role {}", role.displayName),
        NetworkControlCommand::UpdateRole { role } => format!("Update role {}", role.displayName),
        NetworkControlCommand::RemoveRole { roleId } => format!(
            "Remove role {}",
            controlRoleLabel(state, roleId).unwrap_or_else(|_| roleId.clone())
        ),
        NetworkControlCommand::GrantRole { grant: assignment } => format!(
            "Set identity {} on {}",
            controlRoleLabel(state, &assignment.roleId)?,
            controlDeviceLabel(profiles, &assignment.nodeId)?
        ),
        NetworkControlCommand::RevokeRole { nodeId } => {
            format!(
                "Reset identity on {} to the default user",
                controlDeviceLabel(profiles, nodeId)?
            )
        }
        NetworkControlCommand::AdmitMember { nodeId } => {
            format!("Admit device {}", controlDeviceLabel(profiles, nodeId)?)
        }
        NetworkControlCommand::AdmitSpace { sourceSpaceId, identities } => {
            let labels = identities.keys().map(|node| controlDeviceLabel(profiles, node))
                .collect::<Result<Vec<_>, _>>()?;
            format!("Merge Space {sourceSpaceId}; admit {}", labels.join(", "))
        }
        NetworkControlCommand::RemoveMember { nodeId, .. } => {
            format!("Remove device {}", controlDeviceLabel(profiles, nodeId)?)
        }
        NetworkControlCommand::LeaveSpace => "Leave Space".to_string(),
        NetworkControlCommand::DisconnectNode { nodeId } => format!(
            "Disconnect device {}",
            controlDeviceLabel(profiles, nodeId)?
        ),
        NetworkControlCommand::PolicyUpdate { policyId, value } => {
            format!("Update policy {} to {:?}", policyId, value)
        }
    };
    Ok(summary)
}

/// Formats one device using its human-facing name and owner.
fn controlDeviceLabel(
    profiles: &std::collections::BTreeMap<String, CoreSpaceDeviceProfile>,
    nodeId: &str,
) -> Result<String, String> {
    let profile = profiles
        .get(nodeId)
        .ok_or_else(|| format!("device profile is missing for audit device: {nodeId}"))?;
    if profile.userName.trim().is_empty() {
        Ok(profile.displayName.clone())
    } else {
        Ok(format!("{} · {}", profile.displayName, profile.userName))
    }
}

/// Formats one role using its human-facing name.
fn controlRoleLabel(state: &NetworkControlState, roleId: &str) -> Result<String, String> {
    let role = state
        .roles
        .get(roleId)
        .ok_or_else(|| format!("role is missing for audit command: {roleId}"))?;
    Ok(role.displayName.clone())
}

/// Returns the deterministic order used to replay Space control commands.
#[allow(non_snake_case)]
fn controlOperationOrder(operation: &SyncOperation) -> SyncOperationOrder {
    SyncOperationOrder::fromOperation(operation)
}

/// Decodes one structurally valid control operation into its command payload.
#[allow(non_snake_case)]
pub(crate) fn decodeControlOperation(
    operation: &SyncOperation,
) -> Result<NetworkControlCommandRecord, String> {
    validateControlEnvelope(operation)?;
    let record = NetworkControlCommandRecord::deserialize(&operation.payload).map_err(|error| error.to_string())?;
    validateCommandRecord(&record)?;
    if record.commandId != operation.entityId {
        return Err("network control command id does not match its operation entity".to_string());
    }
    Ok(record)
}

/// Validates the immutable synchronization envelope for a Space control command.
#[allow(non_snake_case)]
fn validateControlOperation(operation: &SyncOperation) -> Result<(), String> {
    decodeControlOperation(operation).map(|_| ())
}
fn validateControlEnvelope(operation: &SyncOperation) -> Result<(), String> {
    if operation.domain != NETWORK_CONTROL_SYNC_DOMAIN
        || operation.entityType != NETWORK_CONTROL_ENTITY_TYPE
        || operation.operation != NETWORK_CONTROL_OPERATION
        || operation.semantics != SyncOperationSemantics::Transaction {
        return Err("operation is not a network control command".to_string());
    }
    Ok(())
}

/// Validates the fields carried by one command before it is authorized.
#[allow(non_snake_case)]
fn validateCommandRecord(record: &NetworkControlCommandRecord) -> Result<(), String> {
    validateIdentifier("command id", &record.commandId, 160)?;
    validateSpaceId(&record.spaceId)?;
    validateNodeId(&record.issuerNodeId)?;
    match &record.command {
        NetworkControlCommand::LeaveSpace => Ok(()),
        NetworkControlCommand::Bootstrap { initialAdminNodeId } => {
            validateNodeId(initialAdminNodeId)
        }
        NetworkControlCommand::DefineRole { role } => validateRole(role),
        NetworkControlCommand::UpdateRole { role } => validateRole(role),
        NetworkControlCommand::RemoveRole { roleId } => {
            validateIdentifier("role id", roleId, 120)
        }
        NetworkControlCommand::GrantRole { grant: assignment } => {
            validateIdentityAssignment(assignment)
        }
        NetworkControlCommand::RevokeRole { nodeId } => validateNodeId(nodeId),
        NetworkControlCommand::AdmitSpace { sourceSpaceId, identities } => {
            validateSpaceId(sourceSpaceId)?;
            if sourceSpaceId == &record.spaceId || identities.is_empty() {
                return Err("Space admission requires a distinct source and nonempty membership".into());
            }
            for (nodeId, identityId) in identities {
                validateNodeId(nodeId)?;
                validateIdentifier("role id", identityId, 120)?;
            }
            Ok(())
        }
        NetworkControlCommand::AdmitMember { nodeId }
        | NetworkControlCommand::RemoveMember { nodeId, .. }
        | NetworkControlCommand::DisconnectNode { nodeId } => validateNodeId(nodeId),
        NetworkControlCommand::PolicyUpdate { policyId, value } => {
            validateIdentifier("policy id", policyId, 160)?;
            validateIdentifier("policy value", value, 4_096)
        }
    }
}

/// Authorizes one local command without changing the materialized policy state.
#[allow(non_snake_case)]
fn authorizeCommand(
    state: &NetworkControlState,
    issuerNodeId: &str,
    command: &NetworkControlCommand,
) -> Result<(), String> {
    let mut candidate = state.clone();
    let record = NetworkControlCommandRecord {
        commandId: "local-authorization".to_string(),
        spaceId: state.spaceId.clone(),
        issuerNodeId: issuerNodeId.to_string(),
        command: command.clone(),
    };
    authorizeAndApplyCommand(&mut candidate, &record, issuerNodeId, false)
}

/// Checks issuer provenance and applies a command to the selected policy state when authorized.
#[allow(non_snake_case)]
fn authorizeAndApplyCommand(
    state: &mut NetworkControlState,
    record: &NetworkControlCommandRecord,
    originNodeId: &str,
    legacyAdmission: bool,
) -> Result<(), String> {
    if record.issuerNodeId != originNodeId {
        return Err("network control issuer does not match operation origin".to_string());
    }
    match &record.command {
        NetworkControlCommand::Bootstrap { initialAdminNodeId } => {
            if state.initialized {
                return Err("Space control policy already has an initial administrator".to_string());
            }
            if initialAdminNodeId != &record.issuerNodeId {
                return Err(
                    "initial administrator must issue its own bootstrap command".to_string()
                );
            }
            state.initialized = true;
            state.memberNodeIds.insert(initialAdminNodeId.clone());
            state
                .deviceIdentityIds
                .insert(initialAdminNodeId.clone(), "admin".to_string());
            Ok(())
        }
        NetworkControlCommand::DefineRole { role } => {
            requireGlobalCapability(
                state,
                &record.issuerNodeId,
                NETWORK_IDENTITY_MANAGE_CAPABILITY,
            )?;
            if builtinRoles().contains_key(&role.roleId) {
                return Err("built-in roles cannot be redefined".to_string());
            }
            if state.roles.contains_key(&role.roleId) {
                return Err(format!(
                    "network control role already exists: {}",
                    role.roleId
                ));
            }
            for capability in &role.capabilities {
                requireGlobalCapability(state, &record.issuerNodeId, capability)?;
            }
            state.roles.insert(role.roleId.clone(), role.clone());
            Ok(())
        }
        NetworkControlCommand::UpdateRole { role } => {
            requireGlobalCapability(
                state,
                &record.issuerNodeId,
                NETWORK_IDENTITY_MANAGE_CAPABILITY,
            )?;
            if builtinRoles().contains_key(&role.roleId) {
                return Err("built-in roles cannot be redefined".to_string());
            }
            if !state.roles.contains_key(&role.roleId) {
                return Err(format!(
                    "network control role does not exist: {}",
                    role.roleId
                ));
            }
            for capability in &role.capabilities {
                requireGlobalCapability(state, &record.issuerNodeId, capability)?;
            }
            state.roles.insert(role.roleId.clone(), role.clone());
            Ok(())
        }
        NetworkControlCommand::RemoveRole { roleId } => {
            requireGlobalCapability(
                state,
                &record.issuerNodeId,
                NETWORK_IDENTITY_MANAGE_CAPABILITY,
            )?;
            if builtinRoles().contains_key(roleId) {
                return Err("built-in roles cannot be removed".to_string());
            }
            if !state.roles.contains_key(roleId) {
                return Err(format!("network control role does not exist: {roleId}"));
            }
            // Members keep an identity at all times: devices still assigned
            // to the removed role fall back to the default user identity,
            // mirroring the RevokeRole guard.
            let reassignedNodeIds: Vec<String> = state
                .deviceIdentityIds
                .iter()
                .filter(|(_, identityId)| *identityId == roleId)
                .map(|(nodeId, _)| nodeId.clone())
                .collect();
            for nodeId in reassignedNodeIds {
                state.deviceIdentityIds.insert(nodeId, "user".to_string());
            }
            state.roles.remove(roleId);
            Ok(())
        }
        NetworkControlCommand::GrantRole { grant: assignment } => {
            requireIdentityAssignmentAuthority(state, &record.issuerNodeId, &assignment.roleId)?;
            if !state.memberNodeIds.contains(&assignment.nodeId) {
                return Err("identity target has not been admitted to the Space".to_string());
            }
            if let Some(existingIdentityId) = state.deviceIdentityIds.get(&assignment.nodeId) {
                if assignment.roleId != "admin"
                    && existingIdentityId == "admin"
                    && clearingIdentityLeavesNoAdministrator(state, &assignment.nodeId)
                {
                    return Err("Space control policy must retain an administrator".to_string());
                }
            }
            state
                .deviceIdentityIds
                .insert(assignment.nodeId.clone(), assignment.roleId.clone());
            Ok(())
        }
        NetworkControlCommand::RevokeRole { nodeId } => {
            let identityId = state
                .deviceIdentityIds
                .get(nodeId)
                .ok_or_else(|| format!("network device has no identity: {nodeId}"))?;
            requireGlobalCapability(
                state,
                &record.issuerNodeId,
                NETWORK_IDENTITY_MANAGE_CAPABILITY,
            )?;
            if identityId == "admin" && clearingIdentityLeavesNoAdministrator(state, nodeId) {
                return Err("Space control policy must retain an administrator".to_string());
            }
            // Reset to the default user identity instead of leaving the
            // member identity-less. Every other replay path that grants
            // membership also assigns an identity; an identity-less member
            // would lose the device visibility and routing capabilities its
            // UI depends on and, for a solo Space, any in-band recovery.
            state
                .deviceIdentityIds
                .insert(nodeId.to_string(), "user".to_string());
            Ok(())
        }
        NetworkControlCommand::AdmitMember { nodeId } => {
            requireCapability(
                state,
                &record.issuerNodeId,
                "network.members.join",
                Some(nodeId),
            )?;
            state.disconnectedNodeIds.remove(nodeId);
            state.memberNodeIds.insert(nodeId.clone());
            if !state.deviceIdentityIds.contains_key(nodeId) {
                state
                    .deviceIdentityIds
                    .insert(nodeId.clone(), "user".to_string());
            }
            Ok(())
        }
        NetworkControlCommand::AdmitSpace { identities, .. } => {
            // Validate the complete group and its initial identities before
            // changing membership. Existing target identities remain intact.
            for (nodeId, identityId) in identities {
                requireCapability(state, &record.issuerNodeId, "network.members.join", Some(nodeId))?;
                if !legacyAdmission {
                    requireIdentityAssignmentAuthority(state, &record.issuerNodeId, identityId)?;
                }
            }
            for (nodeId, identityId) in identities {
                state.disconnectedNodeIds.remove(nodeId);
                state.memberNodeIds.insert(nodeId.clone());
                state.deviceIdentityIds.entry(nodeId.clone()).or_insert_with(|| identityId.clone());
            }
            Ok(())
        }
        NetworkControlCommand::RemoveMember { nodeId, .. } => {
            requireCapability(
                state,
                &record.issuerNodeId,
                "network.members.remove",
                Some(nodeId),
            )?;
            if nodeIsSoleAdministrator(state, nodeId) {
                return Err("Space control policy must retain an administrator".to_string());
            }
            // 彻底遗忘：设备回到陌生人状态，不保留任何标记或成员记录。
            // 重新接入只能重新配对并重新申请加入。
            state.deviceIdentityIds.remove(nodeId);
            state.memberNodeIds.remove(nodeId);
            state.disconnectedNodeIds.remove(nodeId);
            Ok(())
        }
        NetworkControlCommand::LeaveSpace => {
            let node = &record.issuerNodeId;
            if !state.memberNodeIds.remove(node) {
                return Err("Only an admitted device can leave its Space".into());
            }
            state.deviceIdentityIds.remove(node);
            state.disconnectedNodeIds.remove(node);
            Ok(())
        }
        NetworkControlCommand::DisconnectNode { nodeId } => {
            requireCapability(
                state,
                &record.issuerNodeId,
                "network.connections.disconnect",
                Some(nodeId),
            )?;
            if !state.memberNodeIds.contains(nodeId) {
                return Err("device has not been admitted to the Space".to_string());
            }
            state.disconnectedNodeIds.insert(nodeId.clone());
            Ok(())
        }
        NetworkControlCommand::PolicyUpdate { policyId, value } => {
            requireGlobalCapability(state, &record.issuerNodeId, "network.policy.update")?;
            state.policies.insert(policyId.clone(), value.clone());
            Ok(())
        }
    }
}

/// Requires a capability independent of a particular managed device.
#[allow(non_snake_case)]
fn requireGlobalCapability(
    state: &NetworkControlState,
    nodeId: &str,
    capability: &str,
) -> Result<(), String> {
    requireCapability(state, nodeId, capability, None)
}

/// Requires one capability for an optional target device.
#[allow(non_snake_case)]
fn requireCapability(
    state: &NetworkControlState,
    nodeId: &str,
    capability: &str,
    targetNodeId: Option<&str>,
) -> Result<(), String> {
    if hasCapability(state, nodeId, capability, targetNodeId) {
        return Ok(());
    }
    Err(format!("node {nodeId} lacks capability {capability}"))
}

/// Admission and later identity changes obey the same assignment policy.
#[allow(non_snake_case)]
fn requireIdentityAssignmentAuthority(
    state: &NetworkControlState,
    issuerNodeId: &str,
    identityId: &str,
) -> Result<(), String> {
    let role = state.roles.get(identityId).ok_or_else(|| {
        format!("network control identity does not exist: {identityId}")
    })?;
    requireGlobalCapability(state, issuerNodeId, NETWORK_IDENTITY_ASSIGN_CAPABILITY)?;
    for capability in &role.capabilities {
        requireGlobalCapability(state, issuerNodeId, capability)?;
    }
    Ok(())
}

/// Resolves one capability from the device's current identity.
#[allow(non_snake_case)]
fn hasCapability(
    state: &NetworkControlState,
    nodeId: &str,
    capability: &str,
    targetNodeId: Option<&str>,
) -> bool {
    if !state.initialized {
        return false;
    }
    let Some(identityId) = state.deviceIdentityIds.get(nodeId) else {
        return false;
    };
    let Some(role) = state.roles.get(identityId) else {
        return false;
    };
    let _ = targetNodeId;
    role.capabilities.contains("*") || role.capabilities.contains(capability)
}

/// Reports whether clearing one device identity would leave the Space without an administrator.
#[allow(non_snake_case)]
fn clearingIdentityLeavesNoAdministrator(state: &NetworkControlState, nodeId: &str) -> bool {
    !state
        .deviceIdentityIds
        .iter()
        .any(|(existingNodeId, identityId)| {
            existingNodeId != nodeId && identityId == "admin"
        })
}

/// Reports whether one device currently owns the only active administrator identity.
#[allow(non_snake_case)]
fn nodeIsSoleAdministrator(state: &NetworkControlState, nodeId: &str) -> bool {
    matches!(state.deviceIdentityIds.get(nodeId), Some(identityId) if identityId == "admin")
        && !state
            .deviceIdentityIds
            .iter()
            .any(|(existingNodeId, identityId)| {
                existingNodeId != nodeId && identityId == "admin"
            })
}

/// Validates one role definition before it is replicated through the control log.
#[allow(non_snake_case)]
fn validateRole(role: &NetworkControlRole) -> Result<(), String> {
    validateIdentifier("role id", &role.roleId, 120)?;
    validateIdentifier("role display name", &role.displayName, 160)?;
    if role.capabilities.is_empty() {
        return Err("network control role must declare at least one capability".to_string());
    }
    for capability in &role.capabilities {
        validateCapability(capability)?;
    }
    Ok(())
}

/// Validates one identity assignment before it is replicated through the control log.
#[allow(non_snake_case)]
fn validateIdentityAssignment(assignment: &NetworkControlIdentityAssignment) -> Result<(), String> {
    validateNodeId(&assignment.nodeId)?;
    validateIdentifier("role id", &assignment.roleId, 120)
}

/// Validates one Space identity carried by a control command.
#[allow(non_snake_case)]
fn validateSpaceId(spaceId: &str) -> Result<(), String> {
    validateIdentifier("space id", spaceId, 160)
}

/// Validates one CoreNode identifier carried by a control command.
#[allow(non_snake_case)]
fn validateNodeId(nodeId: &str) -> Result<(), String> {
    validateIdentifier("node id", nodeId, 160)
}

/// Validates one named capability without parsing or inferring capability syntax.
#[allow(non_snake_case)]
fn validateCapability(capability: &str) -> Result<(), String> {
    validateIdentifier("capability", capability, 160)
}

/// Validates a nonempty bounded text identifier used by the control policy.
#[allow(non_snake_case)]
fn validateIdentifier(fieldName: &str, value: &str, maximumLength: usize) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("network control {fieldName} must not be empty"));
    }
    if trimmed.chars().count() > maximumLength {
        return Err(format!(
            "network control {fieldName} exceeds {maximumLength} characters"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(format!(
            "network control {fieldName} must not contain control characters"
        ));
    }
    Ok(())
}
